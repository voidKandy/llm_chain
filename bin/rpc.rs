use std::sync::LazyLock;

use llm_chain::{telemetry::TRACING, util::json_rpc::thread::RpcListeningThread, MainResult};

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let mut thread = RpcListeningThread::new("127.0.0.1:8000").await?;
    loop {
        if let Some(req) = thread.try_recv().await.unwrap() {}
    }
}
