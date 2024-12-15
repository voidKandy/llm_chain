use clap::{Parser, Subcommand};
use libp2p::identity::Keypair;
use libp2p::{gossipsub, Multiaddr, PeerId};
use llm_chain::node::{client::ClientNode, server::ServerNode, Node, NodeType};
use llm_chain::telemetry::TRACING;
use llm_chain::CHAIN_TOPIC;
use std::sync::LazyLock;
use tracing::warn;

// https://github.com/libp2p/rust-libp2p/tree/master/examples/rendezvous

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
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
                if let Ok(Some(event)) = node.inner.next_event().await {
                    node.inner.handle_event(event).await.unwrap();
                }
            }
        }

        Command::Boot => {
            let mut node = create_boot_node().await?;
            warn!("created node");
            loop {
                if let Ok(Some(event)) = node.inner.next_event().await {
                    node.inner.handle_event(event).await.unwrap();
                }
            }
        }
    }
}

/// Create boot node with private key in boot.key, which was generated with
/// ```shell
/// head -c 32 /dev/urandom > boot.key
/// ```
const BOOT_NODE_PEER_ID: &str = "12D3KooWCwDGQ5jED2DCkdjLpfitvBr6KMDW3VkFLMxE4f67vUen";
const BOOT_NODE_LOCAL_ADDR: &str = "/ip4/127.0.0.1/udp/62649/quic-v1";
const BOOT_NODE_LISTEN_ADDR: &str = "/ip4/0.0.0.0/udp/62649/quic-v1";
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
        .listen_on(BOOT_NODE_LISTEN_ADDR.parse()?)?;

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

    let rendezvous_point_address = BOOT_NODE_LOCAL_ADDR.parse::<Multiaddr>().unwrap();
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
