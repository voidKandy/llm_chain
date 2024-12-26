use clap::Parser;
use client::node::ClientNode;
use core::blockchain::chain::BOOT_NODE_LOCAL_ADDR;
use core::node::Node;
use core::telemetry::TRACING;
use libp2p::identity::Keypair;
use libp2p::{Multiaddr, PeerId};
use std::sync::LazyLock;
use tracing;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'k')]
    key: Option<String>,
    #[arg(short = 'a')]
    rpc_addr: Option<String>,
    #[arg(short = 'd')]
    dial_addr: Option<String>,
}

#[tokio::main]
async fn main() -> core::MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    tracing::warn!("args: {args:#?}");

    let keypair = match args.key {
        Some(path) => {
            let mut bytes = std::fs::read(path).unwrap();
            Keypair::ed25519_from_bytes(&mut bytes)
                .expect("failed to get keypair from boot.key bytes")
        }
        None => Keypair::generate_ed25519(),
    };
    let peer_id = PeerId::from_public_key(&keypair.public());
    tracing::warn!("id: {peer_id:#?}");
    let mut node = Node::<ClientNode>::try_from_keys(
        keypair,
        args.rpc_addr.unwrap_or("127.0.0.1:0".to_string()),
    )
    .await
    .unwrap();

    let boot_node_addr = args
        .dial_addr
        .unwrap_or(BOOT_NODE_LOCAL_ADDR.to_string())
        .parse::<Multiaddr>()
        .unwrap();
    let external_address = "/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>().unwrap();

    node.swarm.add_external_address(external_address);

    node.swarm.dial(boot_node_addr).unwrap();

    node.main_loop().await
}
