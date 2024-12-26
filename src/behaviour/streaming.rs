use std::time::Duration;

use futures::{AsyncReadExt, AsyncWriteExt};
use libp2p::{PeerId, Stream, StreamProtocol};
use rand::RngCore;
use serde::{Deserialize, Serialize};

pub const STREAM_PROTOCOL: StreamProtocol = StreamProtocol::new("/echo");

/// https://github.com/libp2p/rust-libp2p/blob/master/examples/stream/src/main.rs

/// A very simple, `async fn`-based connection handler for our custom echo protocol.
pub async fn connection_handler(peer: PeerId, mut control: libp2p_stream::Control) {
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await; // Wait a second between echos.

        let stream = match control.open_stream(peer, STREAM_PROTOCOL).await {
            Ok(stream) => stream,
            Err(error @ libp2p_stream::OpenStreamError::UnsupportedProtocol(_)) => {
                tracing::info!(%peer, %error);
                return;
            }
            Err(error) => {
                // Other errors may be temporary.
                // In production, something like an exponential backoff / circuit-breaker may be
                // more appropriate.
                tracing::debug!(%peer, %error);
                continue;
            }
        };

        if let Err(e) = send(stream).await {
            tracing::error!(%peer, "Echo send failed: {e}");
            continue;
        }

        tracing::info!(%peer, "Echo complete!")
    }
}

// this should eventually do inference
#[derive(Debug, Deserialize, Serialize)]
enum StreamMessage {
    Open,
    Content(String),
    Close,
}
const MESSAGE_SIZE: usize = size_of::<StreamMessage>() + 1;

pub async fn echo(mut stream: Stream) -> std::io::Result<()> {
    // let mut total = 0;
    let mut buf = [0u8; MESSAGE_SIZE];

    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        // let read = stream.read(&mut buf).await?;
        // let val = serde_json::from_reader(stream)?;

        // total += read;
        let raw = String::from_utf8_lossy(&buf[..n]);
        tracing::warn!("raw: {raw}");
        let val: StreamMessage = serde_json::from_slice(&buf[..n])?;
        tracing::warn!("received {val:?} in echo receive");
        let bytes = serde_json::to_vec(&val).unwrap();
        stream.write_all(&bytes).await.expect("failed to write");
    }
}

async fn send(mut stream: Stream) -> std::io::Result<()> {
    let m = StreamMessage::Content("Hello World".to_string());
    let bytes = serde_json::to_vec(&m).unwrap();

    // let mut bytes = vec![0; MESSAGE_SIZE];
    // rand::thread_rng().fill_bytes(&mut bytes);

    let mut buf = [0u8; MESSAGE_SIZE];
    stream.write_all(&bytes).await.expect("write failed");

    let n = stream.read(&mut buf).await.expect("read failed");
    // let raw = String::from_utf8_lossy(&buf[..n]);
    // tracing::warn!("raw: {raw}");
    let val: StreamMessage = serde_json::from_slice(&buf[..n])?;
    tracing::warn!("received {val:?} in echo receive");

    // if bytes != buf {
    //     return Err(std::io::Error::other("incorrect echo"));
    // }

    stream.close().await?;

    Ok(())
}
