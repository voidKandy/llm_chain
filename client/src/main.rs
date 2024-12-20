pub mod behaviour;
pub mod node;

use clap::{Parser, Subcommand};
use libp2p::identity::Keypair;
use libp2p::{gossipsub, Multiaddr, PeerId};
use llm_chain::blockchain::chain::{
    BOOT_NODE_KEYPAIR, BOOT_NODE_LISTEN_ADDR, BOOT_NODE_LOCAL_ADDR, BOOT_NODE_PEER_ID,
};
use llm_chain::node::{Node, NodeType};
use llm_chain::telemetry::TRACING;
use llm_chain::{MainResult, CHAIN_TOPIC};
use node::ClientNode;
use std::net::IpAddr;
use std::sync::LazyLock;
use tracing;

// https://github.com/libp2p/rust-libp2p/tree/master/examples/rendezvous

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'k')]
    key: Option<String>,
    #[arg(short = 'a')]
    rpc_addr: Option<String>,
}

#[tokio::main]
async fn main() -> llm_chain::MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    tracing::warn!("args: {args:#?}");

    Ok(())
}

async fn create_client_node_and_bootstrap(addr: Option<String>) -> MainResult<Node<ClientNode>> {
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from_public_key(&keypair.public());
    tracing::warn!("id: {peer_id:#?}");
    let mut client_node = Node::try_from_keys(keypair, addr.unwrap_or("127.0.0.1:0".to_string()))
        .await
        .unwrap();

    let rendezvous_point_address = BOOT_NODE_LOCAL_ADDR.parse::<Multiaddr>().unwrap();
    let external_address = "/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>().unwrap();

    client_node.swarm.add_external_address(external_address);

    client_node.swarm.dial(rendezvous_point_address).unwrap();

    Ok(client_node)
}
