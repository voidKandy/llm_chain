pub mod behaviour;
pub mod node;

use clap::{Parser, Subcommand};
use core::blockchain::chain::{BOOT_NODE_KEYPAIR, BOOT_NODE_LISTEN_ADDR};
use core::node::Node;
use core::telemetry::TRACING;
use libp2p::identity::Keypair;
use node::{MinerNode, ProviderNode};
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
    #[arg(short = 'k')]
    key: Option<String>,
    #[arg(short = 'n')]
    net_addr: Option<String>,
    #[arg(short = 'a')]
    rpc_addr: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Provider,
    Miner,
}

#[tokio::main]
async fn main() -> core::MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");
    let keypair = match (args.boot, args.key) {
        (true, opt) => {
            if opt.is_some() {
                warn!("passing a key for a boot node is redundant, the key will not be used");
            }
            let b = BOOT_NODE_KEYPAIR;
            LazyLock::force(&b).to_owned()
        }
        (false, Some(key_path)) => {
            warn!("getting key from {key_path}");
            let mut bytes = std::fs::read(key_path).unwrap();
            Keypair::ed25519_from_bytes(&mut bytes)
                .expect("failed to get keypair from boot.key bytes")
        }
        _ => Keypair::generate_ed25519(),
    };

    match args.command {
        Command::Miner => {
            let mut node = Node::<MinerNode>::try_from_keys(
                keypair.clone(),
                args.rpc_addr.unwrap_or("127.0.0.1:0".to_string()),
            )
            .await
            .unwrap();

            if args.boot {
                node.swarm
                    .listen_on(BOOT_NODE_LISTEN_ADDR.parse().unwrap())
                    .unwrap();
            } else {
                node.swarm
                    .listen_on(
                        args.net_addr
                            .unwrap_or("0.0.0.0:0".to_string())
                            .parse()
                            .unwrap(),
                    )
                    .unwrap();
            }
            node.main_loop().await
        }

        Command::Provider => {
            let mut node = Node::<ProviderNode>::try_from_keys(
                keypair.clone(),
                args.rpc_addr.unwrap_or("127.0.0.1:0".to_string()),
            )
            .await
            .unwrap();
            if args.boot {
                node.swarm
                    .listen_on(BOOT_NODE_LISTEN_ADDR.parse().unwrap())
                    .unwrap();
            } else {
                node.swarm
                    .listen_on(
                        args.net_addr
                            .unwrap_or("0.0.0.0:0".to_string())
                            .parse()
                            .unwrap(),
                    )
                    .unwrap();
            }
            node.main_loop().await
        }
    }
}
