use clap::{Parser, Subcommand};
use libp2p::PeerId;
use llm_chain::{
    blockchain::chain::{BOOT_NODE_KEYPAIR, BOOT_NODE_LISTEN_ADDR, BOOT_NODE_PEER_ID},
    node::Node,
    telemetry::TRACING,
    util::behaviour::NetworkTopic,
    MainResult,
};
use server::node::MinerNode;
use std::sync::LazyLock;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    private_key_path: String,
    #[arg(short = 'a')]
    rpc_addr: Option<String>,
}

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    tracing::warn!("args: {args:#?}");
    let mut node = create_boot_node(args.rpc_addr).await?;
    tracing::warn!("created node");
    node.main_loop().await
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
