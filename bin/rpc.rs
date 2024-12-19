use std::sync::LazyLock;

use llm_chain::{
    telemetry::TRACING,
    util::json_rpc::socket::{next_request, send_response, Response},
    MainResult,
};

#[tokio::main]
async fn main() -> MainResult<()> {
    LazyLock::force(&TRACING);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();
    loop {
        let (stream, addr) = listener.accept().await?;
        tracing::warn!("Connected to {addr:#?}");

        loop {
            if let Some(req) = next_request(&stream).await? {
                tracing::warn!("got req: {req:#?}");
                let response = Response {
                    jsonrpc: "2.0".to_string(),
                    error: None,
                    result: "".to_string(),
                    id: "2".to_string(),
                };
                send_response(&stream, response).await?;
            }
        }
    }
}
