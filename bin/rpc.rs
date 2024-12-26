use clap::{Parser, Subcommand};
use core::node::rpc::RequestWrapper;
use core::{telemetry::TRACING, MainResult};
use seraphic::{socket, RpcRequestWrapper};
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
            Self::PeerCount => RequestWrapper::PeerCount(core::node::rpc::GetPeerCountRequest),
            // obviously, this should be changed later
            Self::GetBal => RequestWrapper::GetBalance(core::node::rpc::GetBalanceRequest {
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
    let stdin = std::io::stdin();
    let mut buf = String::new();
    let mut id = 1;

    let mut req = Into::<RequestWrapper>::into(args.command).into_rpc_request(id);

    let mut stream = TcpStream::connect(args.rpc_addr).await.unwrap();

    loop {
        let bytes = serde_json::to_vec(&req).unwrap();
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

        println!("accepting input: \npeer-count | get-bal | exit");
        stdin.read_line(&mut buf)?;
        let command = match buf.drain(..).collect::<String>().trim() {
            "peer-count" => Command::PeerCount,
            "get-bal" => Command::GetBal,
            "exit" => panic!("exit"),
            _ => {
                tracing::warn!("{buf} is not a valid input");
                continue;
            }
        };
        id += 1;
        req = Into::<RequestWrapper>::into(command).into_rpc_request(id);
    }
}
