//! Reading and writing frames over the socket.
//!
//! The format (a big-endian `u32` header plus JSON) and its ceiling live in
//! [`hoard_core::ipc`], which is pure serde; what is here is the only part that
//! needs a runtime: moving bytes. That split is the ADR's leaf kernel: the
//! contract cannot depend on `tokio`, and the transport can.

use anyhow::{Context, Result};
use hoard_core::ipc::{decode_frame, encode_frame, frame_len, HEADER_BYTES};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Escribe una trama completa. Una sola llamada a `write_all`: dos tramas
/// escritas desde tasks distintas no pueden entrelazarse a medias.
pub async fn write_frame<W, T>(writer: &mut W, message: &T) -> Result<()>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes = encode_frame(message).context("encoding a frame")?;
    writer.write_all(&bytes).await.context("writing a frame")?;
    writer.flush().await.context("flushing a frame")?;
    Ok(())
}

/// Reads a frame. `Ok(None)` means the other end closed cleanly, which for a
/// client that is leaving is normal and not an error worth logging.
pub async fn read_frame<R, T>(reader: &mut R) -> Result<Option<T>>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned,
{
    let mut header = [0u8; HEADER_BYTES];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(anyhow::Error::new(err).context("reading a frame header")),
    }
    // El tope se comprueba antes de reservar: un prefijo de 4 GiB no puede
    // convertirse en una reserva de 4 GiB por mucho que el socket sea local.
    let len = frame_len(header).context("validating a frame header")?;
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .await
        .context("reading a frame body")?;
    Ok(Some(decode_frame(&body).context("decoding a frame")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_core::ipc::{ClientFrame, Hello, Request, PROTOCOL_VERSION};

    #[tokio::test]
    async fn frames_survive_a_round_trip_over_a_pipe() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let hello = ClientFrame::Hello(Hello {
            protocol: PROTOCOL_VERSION,
            client: "test".into(),
        });
        write_frame(&mut client, &hello).await.unwrap();
        write_frame(
            &mut client,
            &ClientFrame::Request {
                id: 1,
                request: Request::Ping,
            },
        )
        .await
        .unwrap();
        drop(client);

        let first: ClientFrame = read_frame(&mut server).await.unwrap().unwrap();
        assert!(matches!(first, ClientFrame::Hello(_)));
        let second: ClientFrame = read_frame(&mut server).await.unwrap().unwrap();
        assert!(matches!(
            second,
            ClientFrame::Request {
                id: 1,
                request: Request::Ping
            }
        ));
        // Cierre limpio, no error.
        let end: Option<ClientFrame> = read_frame(&mut server).await.unwrap();
        assert!(end.is_none());
    }

    /// A header promising more than is allowed is rejected without allocating the
    /// buffer it promises.
    #[tokio::test]
    async fn an_oversized_header_is_rejected() {
        let (mut client, mut server) = tokio::io::duplex(64);
        tokio::spawn(async move {
            let _ = client.write_all(&u32::MAX.to_be_bytes()).await;
            // No body: a reader that tried to read it would hang rather than fail.
            futures_keepalive(client).await;
        });
        let err = read_frame::<_, ClientFrame>(&mut server).await.unwrap_err();
        assert!(err.to_string().contains("frame header"), "{err}");
    }

    /// Mantiene el otro extremo abierto para que el test de arriba falle por el
    /// tope y no por EOF.
    async fn futures_keepalive(stream: tokio::io::DuplexStream) {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        drop(stream);
    }
}
