use std::io::Cursor;

use serde::{Serialize, Deserialize};
use tokio::io::{AsyncRead, AsyncReadExt};

use super::PacketError;

#[derive(Debug)]
pub enum ClipboardStage {
    None,
    Mark1 { id: u8, data: Vec<u8> },
    Mark2 { id: u8, data: Vec<u8> },
    Mark3 { id: u8, data: Vec<u8> },
}

impl ClipboardStage {
    pub fn stage(&self) -> u8 {
        match self {
            ClipboardStage::None => 0,
            ClipboardStage::Mark1 { .. } => 1,
            ClipboardStage::Mark2 { .. } => 2,
            ClipboardStage::Mark3 { .. } => 3,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum ClipboardFormat {
    Text = 0,
    Html = 1,
    Bitmap = 2,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ClipboardData {
    text: Vec<u8>,
    html: Vec<u8>,
    bitmap: Vec<u8>,
}

impl ClipboardData {
    pub fn raw_text(&self) -> &[u8] {
        &self.text
    }

    pub fn raw_html(&self) -> &[u8] {
        &self.html
    }

    pub fn text(&self) -> Option<String> {
        if self.text.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(self.text.as_slice()).to_string())
        }
    }

    pub fn html(&self) -> Option<String> {
        if self.html.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(self.html.as_slice()).to_string())
        }
    }

    pub fn bitmap(&self) -> Option<&[u8]> {
        if self.bitmap.is_empty() {
            None
        } else {
            Some(&self.bitmap)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.html.is_empty() && self.bitmap.is_empty()
    }
}

pub(crate) async fn parse_clipboard(buf: &[u8]) -> Result<ClipboardData, PacketError> {
    let mut stream = Cursor::new(buf);
    let mut ret = ClipboardData::default();
    let _sz = stream.read_u32().await?;
    let num_formats = stream.read_u32().await?;

    for _ in 0..num_formats {
        let format = stream.read_u32().await?;
        let length = stream.read_u32().await? as usize;

        let format = match format {
            0 => ClipboardFormat::Text,
            1 => ClipboardFormat::Html,
            2 => ClipboardFormat::Bitmap,
            _ => Err(PacketError::FormatError)?,
        };

        match format {
            ClipboardFormat::Text => {
                extend_exact(&mut stream, length, &mut ret.text).await?;
            }

            ClipboardFormat::Html => {
                extend_exact(&mut stream, length, &mut ret.html).await?;
            }

            ClipboardFormat::Bitmap => {
                extend_exact(&mut stream, length, &mut ret.bitmap).await?;
            }
        }
    }
    Ok(ret)
}

async fn extend_exact<T: AsyncRead + Send + Unpin>(
    stream: &mut T,
    length: usize,
    buf: &mut Vec<u8>,
) -> Result<(), PacketError> {
    let mut chunk = stream.take(length as u64);
    chunk.read_to_end(buf).await?;
    Ok(())
}
