use std::time::Duration;

use futures::{AsyncReadExt, AsyncWriteExt};
use libp2p::{PeerId, Stream, StreamProtocol};
use rand::RngCore;

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
            tracing::warn!(%peer, "Echo protocol failed: {e}");
            continue;
        }

        tracing::info!(%peer, "Echo complete!")
    }
}

pub async fn echo(mut stream: Stream) -> std::io::Result<usize> {
    let mut total = 0;

    let mut buf = [0u8; 100];

    loop {
        let read = stream.read(&mut buf).await?;
        if read == 0 {
            return Ok(total);
        }

        total += read;
        stream.write_all(&buf[..read]).await?;
    }
}

async fn send(mut stream: Stream) -> std::io::Result<()> {
    let num_bytes = rand::random::<usize>() % 1000;

    let mut bytes = vec![0; num_bytes];
    rand::thread_rng().fill_bytes(&mut bytes);

    stream.write_all(&bytes).await?;

    let mut buf = vec![0; num_bytes];
    stream.read_exact(&mut buf).await?;

    if bytes != buf {
        return Err(std::io::Error::other("incorrect echo"));
    }

    stream.close().await?;

    Ok(())
}
