#[cfg(feature = "client")]
#[tokio::main]
#[cfg(not(feature = "client"))]
async fn main() -> llm_chain::MainResult<()> {
    eprintln!("This binary requieres the client feature");
    Ok(())
}
