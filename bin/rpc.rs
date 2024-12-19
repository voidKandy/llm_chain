use clap::{Parser, Subcommand};
use llm_chain::node::rpc::RequestMethod;
use llm_chain::util::json_rpc::socket;
use llm_chain::{telemetry::TRACING, MainResult};
use std::sync::LazyLock;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
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

    let mut ready = stream
        .ready(Interest::READABLE | Interest::WRITABLE)
        .await
        .unwrap();

    if ready.is_writable() {
        stream.write_all(&bytes).await.unwrap();
        warn!("sent request: {req:#?}");

        if !ready.is_readable() {
            ready = stream.ready(Interest::READABLE).await.unwrap();
        }

        if ready.is_readable() {
            let mut buf = [0u8; 1024];
            let n = stream.read(&mut buf).await.unwrap();
            let res: socket::Response = serde_json::from_slice(&buf[..n]).unwrap();

            warn!("got response: {res:#?}")
        }
    }

    Ok(())
}
