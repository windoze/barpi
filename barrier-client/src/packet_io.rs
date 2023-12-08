use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::PacketError;

#[async_trait]
pub trait PacketReader: AsyncRead + Send + Unpin {
    async fn consume_bytes(&mut self, mut len: usize) -> Result<(), PacketError> {
        let mut buf = [0; 16];
        while len > 0 {
            let to_read = core::cmp::min(len, buf.len());
            self.read_exact(&mut buf[..to_read]).await?;
            len -= to_read;
        }
        Ok(())
    }

    async fn discard_exact(&mut self, len: usize) -> Result<(), PacketError> {
        let mut buf = [0; 16];
        let mut len = len;
        while len > 0 {
            let to_read = core::cmp::min(len, buf.len());
            self.read_exact(&mut buf[..to_read]).await?;
            len -= to_read;
        }
        Ok(())
    }

    async fn read_packet_size(&mut self) -> Result<u32, PacketError> {
        Ok(self.read_u32().await?)
    }

    async fn read_bytes_fixed<const N: usize>(&mut self) -> Result<[u8; N], PacketError> {
        let mut res = [0; N];
        self.read_exact(&mut res).await?;
        Ok(res)
    }
}

impl<T: AsyncRead + Send + Unpin> PacketReader for T {}

#[async_trait]
pub trait PacketWriter: AsyncWrite + Send + Unpin {
    async fn write_str(&mut self, data: &str) -> Result<(), PacketError> {
        self.write_u32(data.len() as u32).await?;
        self.write_all(data.as_bytes()).await?;
        Ok(())
    }
}

impl<T: AsyncWrite + Send + Unpin> PacketWriter for T {}
