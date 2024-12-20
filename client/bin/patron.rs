use clap::Parser;
use client::node::ClientNode;
use libp2p::{identity::Keypair, Multiaddr, PeerId};
use llm_chain::{
    blockchain::chain::BOOT_NODE_LOCAL_ADDR, node::Node, telemetry::TRACING, MainResult,
};
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
    let mut node = create_client_node_and_bootstrap(args.rpc_addr).await?;
    tracing::warn!("created node");
    node.main_loop().await
}

async fn create_client_node_and_bootstrap(
    addr: Option<String>,
) -> llm_chain::MainResult<Node<ClientNode>> {
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from_public_key(&keypair.public());
    tracing::warn!("id: {peer_id:#?}");
    let mut client_node =
        Node::<ClientNode>::try_from_keys(keypair, addr.unwrap_or("127.0.0.1:0".to_string()))
            .await
            .unwrap();

    let rendezvous_point_address = BOOT_NODE_LOCAL_ADDR.parse::<Multiaddr>().unwrap();
    let external_address = "/ip4/0.0.0.0/udp/0/quic-v1".parse::<Multiaddr>().unwrap();

    client_node.swarm.add_external_address(external_address);

    client_node.swarm.dial(rendezvous_point_address).unwrap();

    Ok(client_node)
}
