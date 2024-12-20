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
use llm_chain::util::behaviour::NetworkTopic;
use llm_chain::CHAIN_TOPIC;
use node::MinerNode;
use std::net::IpAddr;
use std::sync::LazyLock;
use tracing::warn;

// https://github.com/libp2p/rust-libp2p/tree/master/examples/rendezvous

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(short = 'b')]
    boot: bool,
    #[arg(short = 'a')]
    rpc_addr: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Provider,
    Miner,
}

#[tokio::main]
async fn main() -> llm_chain::MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");

    match args.command {
        Command::Miner => {
            // let mut node = create_client_node_and_bootstrap(args.rpc_addr).await?;
            // node.main_loop().await
        }

        Command::Provider => {
            // let mut node = create_boot_node(args.rpc_addr).await?;
            // warn!("created node");
            // node.main_loop().await
        }
    }

    Ok(())
}

async fn create_boot_node(addr: Option<String>) -> llm_chain::MainResult<Node<MinerNode>> {
    let b = BOOT_NODE_KEYPAIR;
    let keypair = LazyLock::force(&b);
    let peer_id = PeerId::from_public_key(&keypair.public());

    assert_eq!(peer_id.to_string().as_str(), BOOT_NODE_PEER_ID);
    tracing::warn!("id: {peer_id:#?}");
    let mut server_node = Node::<MinerNode>::try_from_keys(
        keypair.clone(),
        addr.unwrap_or("127.0.0.1:0".to_string()),
    )
    .await
    .unwrap();
    server_node
        .swarm
        .behaviour_mut()
        .shared
        .gossip
        .subscribe(&NetworkTopic::ChainUpdate.subscribe())?;

    server_node
        .swarm
        .listen_on(BOOT_NODE_LISTEN_ADDR.parse()?)?;

    Ok(server_node)
}
