use clap::{Parser, Subcommand};
use libp2p::identity::Keypair;
use libp2p::{gossipsub, Multiaddr, PeerId};
use llm_chain::blockchain::chain::{
    BOOT_NODE_KEYPAIR, BOOT_NODE_LISTEN_ADDR, BOOT_NODE_LOCAL_ADDR, BOOT_NODE_PEER_ID,
};
use llm_chain::node::{client::ClientNode, validator::ValidatorNode, Node, NodeType};
use llm_chain::telemetry::TRACING;
use llm_chain::CHAIN_TOPIC;
use std::sync::LazyLock;
use tracing::warn;

// https://github.com/libp2p/rust-libp2p/tree/master/examples/rendezvous

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
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
            node.main_loop().await
        }

        Command::Boot => {
            let mut node = create_boot_node().await?;
            warn!("created node");
            node.main_loop().await
        }
    }
}

async fn create_boot_node() -> llm_chain::MainResult<Node<ValidatorNode>> {
    let b = BOOT_NODE_KEYPAIR;
    let keypair = LazyLock::force(&b);
    let peer_id = PeerId::from_public_key(&keypair.public());

    assert_eq!(peer_id.to_string().as_str(), BOOT_NODE_PEER_ID);
    warn!("id: {peer_id:#?}");

    let mut server_node = Node::<ValidatorNode>::try_from_keys(keypair.clone()).unwrap();
    let chain_topic = gossipsub::IdentTopic::new(CHAIN_TOPIC);
    server_node
        .swarm
        .behaviour_mut()
        .shared
        .gossip
        .subscribe(&chain_topic)?;

    server_node
        .swarm
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
        .swarm
        .behaviour_mut()
        .shared
        .gossip
        .subscribe(&chain_topic)?;

    let rendezvous_point_address = BOOT_NODE_LOCAL_ADDR.parse::<Multiaddr>().unwrap();
    let external_address = "/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>().unwrap();

    client_node.swarm.add_external_address(external_address);

    client_node.swarm.dial(rendezvous_point_address).unwrap();

    Ok(client_node)
}
