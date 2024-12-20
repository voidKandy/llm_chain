use clap::{Parser, Subcommand};
use llm_chain::node::rpc::RequestWrapper;
use llm_chain::util::json_rpc::{socket, RpcRequestWrapper};
use llm_chain::{telemetry::TRACING, MainResult};
use rand::Rng;
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
    GetBal,
}

impl Into<RequestWrapper> for Command {
    fn into(self) -> RequestWrapper {
        match self {
            Self::PeerCount => RequestWrapper::PeerCount(llm_chain::node::rpc::GetPeerCountRequest),
            // obviously, this should be changed later
            Self::GetBal => RequestWrapper::GetBalance(llm_chain::node::rpc::GetBalanceRequest {
                address: "".to_string(),
            }),
        }
    }
}

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let args = Args::parse();
    warn!("args: {args:#?}");

    let mut rng = rand::thread_rng();
    // for now req will have random ids
    let id: u32 = rng.gen();

    let req = Into::<RequestWrapper>::into(args.command).into_rpc_request(id);
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
