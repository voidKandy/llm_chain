use clap::Parser;
use libp2p::Multiaddr;
use llm_chain::chain::block::init_chain;
use llm_chain::node::{client::ClientNode, provider::ProviderNode, validator::ValidatorNode, Node};
use llm_chain::telemetry::TRACING;
use std::sync::LazyLock;
use tracing::warn;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(name = "Dial Address")]
    pub dial_addr: Option<String>,
    /// if true, provider
    /// if false, validator
    /// if none, client
    #[arg(short = 's')]
    pub server: Option<bool>,
}

#[tokio::main]
async fn main() -> llm_chain::MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");
    let dial_address = args.dial_addr.and_then(|add| add.parse::<Multiaddr>().ok());
    match args.server {
        Some(true) => {
            let mut node = Node::<ProviderNode>::init(dial_address, init_chain())?;
            return node.main_loop().await;
        }
        Some(false) => {
            let mut node = Node::<ValidatorNode>::init(dial_address, init_chain())?;
            return node.main_loop().await;
        }
        None => {
            let mut node = Node::<ClientNode>::init(dial_address, init_chain())?;
            return node.main_loop().await;
        }
    };
}
