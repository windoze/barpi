use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

use super::PacketError;

#[async_trait]
pub trait PacketReader: AsyncRead + Send + Unpin {
    async fn consume_bytes(&mut self, mut len: usize) -> Result<(), PacketError> {
        let mut buf = [0; 16];
        while len > 0 {
            let to_read = std::cmp::min(len, buf.len());
            self.read_exact(&mut buf[..to_read]).await?;
            len -= to_read;
        }
        Ok(())
    }

    async fn discard_exact(&mut self, len: usize) -> Result<(), PacketError> {
        let mut buf = [0; 16];
        let mut len = len;
        while len > 0 {
            let to_read = std::cmp::min(len, buf.len());
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

    async fn read_bytes(&mut self) -> Result<Vec<u8>, PacketError> {
        let mut buf = vec![];

        let len = self.read_u32().await?;

        let mut chunk =
            self.take(u64::try_from(len).map_err(|_| PacketError::InsufficientDataError)?);
        chunk.read_to_end(&mut buf).await?;

        Ok(buf)
    }

    async fn read_str_lit(&mut self, lit: &str) -> Result<(), PacketError> {
        let mut buf = vec![];

        let mut chunk =
            self.take(u64::try_from(lit.len()).map_err(|_| PacketError::InsufficientDataError)?);
        chunk.read_to_end(&mut buf).await?;

        if buf == lit.as_bytes() {
            Ok(())
        } else {
            Err(PacketError::FormatError)
        }
    }

    // async fn read_i8(&mut self) -> Result<i8, PacketError> {
    //     let mut buf = [0; 1];
    //     self.read_exact(&mut buf).await?;
    //     Ok(buf[0] as i8)
    // }

    // async fn read_u8(&mut self) -> Result<u8, PacketError> {
    //     let mut buf = [0; 1];
    //     self.read_exact(&mut buf).await?;
    //     Ok(buf[0])
    // }

    // async fn read_i16(&mut self) -> Result<i16, PacketError> {
    //     let mut buf = [0; 2];
    //     self.read_exact(&mut buf).await?;
    //     Ok(i16::from_be_bytes(buf))
    // }

    // async fn read_u16(&mut self) -> Result<u16, PacketError> {
    //     let mut buf = [0; 2];
    //     self.read_exact(&mut buf).await?;
    //     Ok(u16::from_be_bytes(buf))
    // }

    // async fn read_u32(&mut self) -> Result<u32, PacketError> {
    //     let mut buf = [0; 4];
    //     self.read_exact(&mut buf).await?;
    //     Ok(u32::from_be_bytes(buf))
    // }
}

impl<T: AsyncRead + Send + Unpin> PacketReader for T {}

#[async_trait]
pub trait PacketWriter: AsyncWrite + Send + Unpin {
    async fn write_str(&mut self, data: &str) -> Result<(), PacketError> {
        self.write_u32(data.len() as u32).await?;
        self.write_all(data.as_bytes()).await?;
        Ok(())
    }

    // async fn write_u16(&mut self, data: u16) -> Result<(), PacketError> {
    //     Ok(self.write_all(&data.to_be_bytes()).await?)
    // }

    // async fn write_u32(&mut self, data: u32) -> Result<(), PacketError> {
    //     Ok(self.write_all(&data.to_be_bytes()).await?)
    // }
}

impl<T: AsyncWrite + Send + Unpin> PacketWriter for T {}
