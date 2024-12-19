use clap::{Parser, Subcommand};
use llm_chain::node::rpc::RequestMethod;
use llm_chain::{telemetry::TRACING, MainResult};
use std::sync::LazyLock;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::warn;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(short = 'a')]
    rpc_addr: String,
}

#[derive(Subcommand, Debug)]
enum Command {
    PeerCount,
}

impl Into<RequestMethod> for Command {
    fn into(self) -> RequestMethod {
        match self {
            Self::PeerCount => RequestMethod::PeerCount(llm_chain::node::rpc::GetPeerCountRequest),
        }
    }
}

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");
    let req = Into::<RequestMethod>::into(args.command).into_socket_request(1, "2.0");
    let bytes = serde_json::to_vec(&req).unwrap();

    let mut stream = TcpStream::connect(args.rpc_addr).await.unwrap();
    stream.write_all(&bytes).await.unwrap();
    Ok(())
}
