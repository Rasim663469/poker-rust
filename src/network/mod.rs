pub mod client;
pub mod protocol;
pub mod server;

use serde::{de::DeserializeOwned, Serialize};
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn send_json<T, W>(writer: &mut W, message: &T) -> io::Result<()>
where
    T: Serialize,
    W: AsyncWriteExt + Unpin,
{
    let payload = serde_json::to_vec(message)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("serde write: {e}")))?;
    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(&payload).await?;
    Ok(())
}

pub async fn recv_json<T, R>(reader: &mut R) -> io::Result<T>
where
    T: DeserializeOwned,
    R: AsyncReadExt + Unpin,
{
    let len = reader.read_u32().await? as usize;
    if len == 0 || len > 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid frame length: {len}"),
        ));
    }
    let mut payload = vec![0_u8; len];
    reader.read_exact(&mut payload).await?;
    serde_json::from_slice(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("serde read: {e}")))
}
