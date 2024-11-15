mod telemetry;
use clap::Parser;
use futures::StreamExt;
use libp2p::{
    gossipsub::{self, MessageAuthenticity},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore, Mode, QueryId, QueryResult, RecordKey},
    swarm::{NetworkBehaviour, SwarmEvent},
    Multiaddr, PeerId, StreamProtocol, Swarm,
};
use serde::{Deserialize, Serialize};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
    sync::LazyLock,
    time::Duration,
};
use telemetry::TRACING;
use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};
use tracing::warn;

type MainResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(name = "Dial Address")]
    pub dial_addr: Option<String>,
    #[arg(short = 's')]
    pub server: bool,
}

static KEYS: LazyLock<Keypair> = LazyLock::new(|| Keypair::generate_ed25519());
static PEER_ID: LazyLock<PeerId> = LazyLock::new(|| PeerId::from(KEYS.public()));

// these should be mapped to real models down the line
const MODEL_ID_0: &str = "model_0";
const MODEL_ID_1: &str = "model_1";
#[derive(Debug, Deserialize, Serialize)]
enum Message<'m> {
    CompletionReq { model_id: u8, prompt: &'m str },
}

#[derive(NetworkBehaviour)]
struct SysBehaviour {
    gossip: gossipsub::Behaviour,
    kad: kad::Behaviour<MemoryStore>,
    identify: identify::Behaviour,
}

#[derive(Debug, Default)]
struct ClientNodeData {
    stdin: Option<Lines<BufReader<Stdin>>>,
    model_prompt: Option<String>,
    provider_query: Option<QueryId>,
}

const IDENTIFY_ID: &str = "/ipfs/id/1.0.0";
impl SysBehaviour {
    fn new(peer_id: PeerId, key: Keypair) -> Self {
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };
        let identify =
            identify::Behaviour::new(identify::Config::new(IDENTIFY_ID.to_string(), key.public()));

        let peer_store = MemoryStore::new(peer_id);
        let gossip_config = gossipsub::ConfigBuilder::default()
            .message_id_fn(message_id_fn)
            .build()
            .expect("failed to build gossip config");

        let gossip =
            gossipsub::Behaviour::new(MessageAuthenticity::Signed(key), gossip_config).unwrap();

        let mut kad_config = kad::Config::new(StreamProtocol::new("/idontknowhwhattocallthis"));

        // Good for debugging, by default this is set to 5 mins
        // kad_config.set_periodic_bootstrap_interval(Some(Duration::from_secs(10)));

        let kad = kad::Behaviour::<MemoryStore>::with_config(peer_id, peer_store, kad_config);

        SysBehaviour {
            gossip,
            kad,
            identify,
        }
    }
}

async fn read_stdin_opt(stdinopt: &mut Option<Lines<BufReader<Stdin>>>) -> Option<String> {
    if let Some(stdin) = stdinopt.as_mut() {
        return stdin.next_line().await.expect("failed to read stdin");
    }
    None
}

async fn handle_swarm_event(
    swarm: &mut Swarm<SysBehaviour>,
    client_node_data: &mut ClientNodeData,
    event: SwarmEvent<SysBehaviourEvent>,
) {
    match event {
        SwarmEvent::NewListenAddr { address, .. } => warn!("Listening on {address:?}"),
        SwarmEvent::Behaviour(beh_event) => {
            match beh_event {
                SysBehaviourEvent::Gossip(gossipsub::Event::Message {
                    propagation_source,
                    message_id,
                    message,
                }) => {
                    let deserialized: Message =
                        serde_json::from_slice(&message.data).expect("failed to deserialize");

                    warn!("Got message: '{deserialized:#?}' with id: {message_id} from peer: {propagation_source}");
                }
                SysBehaviourEvent::Kad(kad_e) => match kad_e {
                    kad::Event::OutboundQueryProgressed {
                        id,
                        result,
                        stats,
                        step,
                    } => {
                        // if client_node_data.provider_query == Some(id) {
                        //     warn!("client node found a provider:\nresult={result:#?}\nstats={stats:#?}");
                        // }
                        match result {
                            QueryResult::StartProviding(res) => {
                                warn!("started providing: {res:#?}");
                            }
                            QueryResult::GetProviders(res) => {
                                warn!("get providers result: {res:#?}");
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        warn!("kad event: {kad_e:#?}");
                    }
                },
                _ => {
                    warn!("unhandled behavior event: {beh_event:#?}");
                }
            }
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            warn!("New connection to peer: {peer_id:#?}")
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            warn!("Closed connection to peer: {peer_id:#?}")
        }
        event => {
            tracing::error!("Unhandled Event: {event:#?}");
        }
    }
}

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");
    let local_id = LazyLock::force(&PEER_ID);

    let mut client_node_data = ClientNodeData::default();

    warn!("Peer Id: {}", &local_id);

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::tls::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_behaviour(|key| SysBehaviour::new(*local_id, key.clone()))?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
        .build();

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossip.subscribe(&topic)?;
    let key = RecordKey::new(&MODEL_ID_0);

    if args.server {
        swarm.behaviour_mut().kad.set_mode(Some(Mode::Server));

        let qid = swarm
            .behaviour_mut()
            .kad
            .start_providing(key.clone())
            .expect("failed to make node a provider");

        warn!("making this node a server, qid: {qid:#?}");
    } else {
        client_node_data.stdin = Some(tokio::io::BufReader::new(tokio::io::stdin()).lines());
    }

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    if let Some(addr) = args.dial_addr {
        let remote: Multiaddr = addr.parse()?;
        swarm.dial(remote)?;
        warn!("Dialed {addr}")
    } else {
    }

    loop {
        tokio::select! {
            Some(line) = read_stdin_opt(&mut client_node_data.stdin) => {
                client_node_data.provider_query= Some(swarm
                    .behaviour_mut()
                    .kad
                    .get_providers(key.clone()));
                client_node_data.model_prompt = Some(line);
                warn!("updated client node: {client_node_data:#?}");

                // let message = Message::CompletionReq{model_id: MODEL_ID_0, prompt: &line};
                // let json_byte_vec = serde_json::to_vec(&message).expect("failed serialization of message");
                // if let Err(e) = swarm
                //     .behaviour_mut().gossip
                //     .publish(topic.clone(), json_byte_vec) {
                //     warn!("Publish error: {e:?}");
                // }
            }
            event = swarm.select_next_some() => handle_swarm_event(&mut swarm, &mut client_node_data, event).await
        }
    }
}
