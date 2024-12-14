use clap::{Parser, Subcommand};
use futures::StreamExt;
use libp2p::gossipsub::MessageAuthenticity;
use libp2p::identity::Keypair;
use libp2p::request_response::ProtocolSupport;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{
    gossipsub, identify, rendezvous, request_response, Multiaddr, PeerId, StreamProtocol, Swarm,
};
use llm_chain::behavior::gossip::{self, CompConnect, CompConnectConfirm};
use llm_chain::behavior::IDENTIFY_ID;
use llm_chain::telemetry::TRACING;
use llm_chain::{MainResult, CHAIN_TOPIC};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::warn;

// https://github.com/libp2p/rust-libp2p/tree/master/examples/rendezvous

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Boot,
    Client,
}

#[tokio::main]
async fn main() -> llm_chain::MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");

    match args.command {
        Command::Client => {
            let mut node = create_client_node_and_bootstrap().await?;
            loop {
                let event = node.inner.next_event().await;
                warn!("event: {event:#?}");
            }
        }

        Command::Boot => {
            let mut node = create_boot_node().await?;
            loop {
                let event = node.inner.next_event().await;
                warn!("event: {event:#?}");
            }
        }
    }
}

pub struct Node<T> {
    keys: Keypair,
    inner: T,
}

impl<T> Node<T>
where
    T: NodeType,
{
    fn try_from_keys(keys: Keypair) -> MainResult<Self> {
        let swarm = T::swarm(keys.clone())?;
        let inner = T::from_swarm(swarm);
        Ok(Self { inner, keys })
    }
}

enum NodeEvent<B: NetworkBehaviour, E: NodeTypeEvent> {
    Swarm(SwarmEvent<B::ToSwarm>),
    NodeType(E),
}

impl<B: NetworkBehaviour, E: NodeTypeEvent> std::fmt::Debug for NodeEvent<B, E>
where
    B::ToSwarm: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeType(e) => write!(f, "{e:?}"),
            Self::Swarm(e) => write!(f, "{e:?}"),
        }
    }
}

impl<B: NetworkBehaviour, E: NodeTypeEvent> From<SwarmEvent<B::ToSwarm>> for NodeEvent<B, E> {
    fn from(value: SwarmEvent<B::ToSwarm>) -> Self {
        Self::Swarm(value)
    }
}

impl<B: NetworkBehaviour, E: NodeTypeEvent> From<E> for NodeEvent<B, E> {
    fn from(value: E) -> Self {
        Self::NodeType(value)
    }
}

trait NodeTypeEvent: std::fmt::Debug {}
trait NodeType {
    type Behaviour: NetworkBehaviour;
    type Event: NodeTypeEvent;
    fn behaviour(keys: &Keypair) -> Self::Behaviour;

    fn swarm_mut(&mut self) -> &mut Swarm<Self::Behaviour>;
    fn swarm(keys: Keypair) -> MainResult<Swarm<Self::Behaviour>> {
        Ok(libp2p::SwarmBuilder::with_existing_identity(keys.clone())
            .with_tokio()
            .with_quic()
            .with_dns()?
            .with_behaviour(|key| Self::behaviour(key))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
            })
            .build())
    }

    async fn next_event(&mut self) -> MainResult<Option<NodeEvent<Self::Behaviour, Self::Event>>>;

    fn from_swarm(swarm: Swarm<Self::Behaviour>) -> Self
    where
        Self: Sized;
}

pub struct ServerNode {
    swarm: Swarm<ServerNodeBehavior>,
}

#[derive(NetworkBehaviour)]
struct ServerNodeBehavior {
    pub gossip: gossipsub::Behaviour,
    pub rendezvous: rendezvous::server::Behaviour,
    pub identify: identify::Behaviour,
    pub req_res: gossip::CompReqRes,
}

#[derive(Debug)]
enum ServerNodeEvent {}
impl NodeTypeEvent for ServerNodeEvent {}

impl NodeType for ServerNode {
    type Behaviour = ServerNodeBehavior;
    type Event = ServerNodeEvent;

    fn from_swarm(swarm: Swarm<ServerNodeBehavior>) -> Self
    where
        Self: Sized,
    {
        Self { swarm }
    }

    fn swarm_mut(&mut self) -> &mut Swarm<ServerNodeBehavior> {
        &mut self.swarm
    }

    async fn next_event(&mut self) -> MainResult<Option<NodeEvent<Self::Behaviour, Self::Event>>> {
        tokio::select! {
            swarm_event = self.swarm.select_next_some() => Ok(Some(NodeEvent::from(swarm_event)))
        }
    }

    fn behaviour(keys: &Keypair) -> ServerNodeBehavior {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify = identify::Behaviour::new(identify::Config::new(
            IDENTIFY_ID.to_string(),
            keys.public(),
        ));

        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(keys.clone()), gossip_config)
                .unwrap();

        let req_res = request_response::json::Behaviour::<CompConnect, CompConnectConfirm>::new(
            [(
                StreamProtocol::new("/compreqres/1.0.0"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );

        ServerNodeBehavior {
            gossip,
            rendezvous: rendezvous::server::Behaviour::new(rendezvous::server::Config::default()),
            identify,
            req_res,
        }
    }
}

use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
pub struct ClientNode {
    swarm: Swarm<ClientNodeBehavior>,
    stdin: Lines<BufReader<Stdin>>,
}

#[derive(NetworkBehaviour)]
struct ClientNodeBehavior {
    pub gossip: gossipsub::Behaviour,
    pub rendezvous: rendezvous::client::Behaviour,
    pub identify: identify::Behaviour,
    pub req_res: gossip::CompReqRes,
}

#[derive(Debug)]
enum ClientNodeEvent {
    UserInput(String),
}
impl NodeTypeEvent for ClientNodeEvent {}

impl NodeType for ClientNode {
    type Behaviour = ClientNodeBehavior;
    type Event = ClientNodeEvent;
    fn swarm_mut(&mut self) -> &mut Swarm<ClientNodeBehavior> {
        &mut self.swarm
    }
    fn from_swarm(swarm: Swarm<ClientNodeBehavior>) -> Self
    where
        Self: Sized,
    {
        Self {
            swarm,
            stdin: tokio::io::BufReader::new(tokio::io::stdin()).lines(),
        }
    }

    async fn next_event(&mut self) -> MainResult<Option<NodeEvent<Self::Behaviour, Self::Event>>> {
        tokio::select! {
             swarm_event = self.swarm.select_next_some() => Ok(Some(NodeEvent::from(swarm_event))),
            Ok(Some(line)) = self.stdin.next_line() => Ok(Some(ClientNodeEvent::UserInput(line).into())),

        }
    }

    fn behaviour(keys: &Keypair) -> ClientNodeBehavior {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify = identify::Behaviour::new(identify::Config::new(
            IDENTIFY_ID.to_string(),
            keys.public(),
        ));

        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(keys.clone()), gossip_config)
                .unwrap();

        let req_res = request_response::json::Behaviour::<CompConnect, CompConnectConfirm>::new(
            [(
                StreamProtocol::new("/compreqres/1.0.0"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );

        ClientNodeBehavior {
            gossip,
            rendezvous: rendezvous::client::Behaviour::new(keys.clone()),
            identify,
            req_res,
        }
    }
}

/// Create boot node with private key in boot.key, which was generated with
/// ```shell
/// head -c 32 /dev/urandom > boot.key
/// ```
const BOOT_NODE_PEER_ID: &str = "12D3KooWCwDGQ5jED2DCkdjLpfitvBr6KMDW3VkFLMxE4f67vUen";
const BOOT_NODE_ADDR: &str = "/ip4/127.0.0.1/udp/62649/quic-v1";
async fn create_boot_node() -> llm_chain::MainResult<Node<ServerNode>> {
    let mut bytes = std::fs::read("boot.key").unwrap();
    let keypair = Keypair::ed25519_from_bytes(&mut bytes)?;
    let peer_id = PeerId::from_public_key(&keypair.public());
    assert_eq!(peer_id.to_string().as_str(), BOOT_NODE_PEER_ID);
    warn!("id: {peer_id:#?}");

    let mut server_node = Node::<ServerNode>::try_from_keys(keypair).unwrap();
    let chain_topic = gossipsub::IdentTopic::new(CHAIN_TOPIC);
    server_node
        .inner
        .swarm_mut()
        .behaviour_mut()
        .gossip
        .subscribe(&chain_topic)?;

    server_node
        .inner
        .swarm_mut()
        .listen_on(BOOT_NODE_ADDR.parse()?)?;

    Ok(server_node)
}

async fn create_client_node_and_bootstrap() -> llm_chain::MainResult<Node<ClientNode>> {
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from_public_key(&keypair.public());
    warn!("id: {peer_id:#?}");
    let mut client_node = Node::<ClientNode>::try_from_keys(keypair).unwrap();

    let chain_topic = gossipsub::IdentTopic::new(CHAIN_TOPIC);
    client_node
        .inner
        .swarm_mut()
        .behaviour_mut()
        .gossip
        .subscribe(&chain_topic)?;

    let rendezvous_point_address = BOOT_NODE_ADDR.parse::<Multiaddr>().unwrap();
    let external_address = "/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>().unwrap();

    client_node
        .inner
        .swarm_mut()
        .add_external_address(external_address);

    client_node
        .inner
        .swarm_mut()
        .dial(rendezvous_point_address)
        .unwrap();

    Ok(client_node)
}
