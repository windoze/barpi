use tokio::io::{AsyncWrite, AsyncWriteExt};

#[cfg(feature = "clipboard")]
use crate::ClipboardData;

use super::{PacketError, PacketWriter};

#[allow(dead_code)]
#[derive(Debug)]
pub enum Packet {
    QueryInfo,
    DeviceInfo {
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        _dummy: u16,
        mx: u16, // x position of the mouse on the secondary screen
        my: u16, // y position of the mouse on the secondary screen
    },
    InfoAck,
    KeepAlive,
    ClientNoOp,
    #[cfg(feature = "barrier-options")]
    ResetOptions,
    #[cfg(feature = "barrier-options")]
    SetDeviceOptions(std::collections::HashMap<String, u32>),
    ErrorUnknownDevice,
    GrabClipboard {
        id: u8,
        seq_num: u32,
    },
    #[cfg(feature = "clipboard")]
    SetClipboard {
        id: u8,
        data: ClipboardData,
    },
    CursorEnter {
        x: u16,
        y: u16,
        seq_num: u32,
        mask: u16,
    },
    MouseUp {
        id: i8,
    },
    MouseDown {
        id: i8,
    },
    KeyUp {
        id: u16,
        mask: u16,
        button: u16,
    },
    KeyDown {
        id: u16,
        mask: u16,
        button: u16,
    },
    KeyRepeat {
        id: u16,
        mask: u16,
        button: u16,
        count: u16,
    },
    MouseWheel {
        x_delta: i16,
        y_delta: i16,
    },
    CursorLeave,
    MouseMoveAbs {
        x: u16,
        y: u16,
    },
    MouseMove {
        x: i16,
        y: i16,
    },
    Unknown([u8; 4]),
}

impl Packet {
    pub async fn write_wire<W: AsyncWrite + Send + Unpin>(
        self,
        mut out: W,
    ) -> Result<(), PacketError> {
        match self {
            Packet::QueryInfo => {
                out.write_str("QINF").await?;
                Ok(())
            }
            Packet::DeviceInfo {
                x,
                y,
                w,
                h,
                _dummy,
                mx,
                my,
            } => {
                let mut buf = [0u8; 4 + 2 * 7 + 4];
                buf[0..4].copy_from_slice((4 + 2u32 * 7).to_be_bytes().as_ref());
                buf[4..8].copy_from_slice(b"DINF");
                buf[8..10].copy_from_slice(x.to_be_bytes().as_ref());
                buf[10..12].copy_from_slice(y.to_be_bytes().as_ref());
                buf[12..14].copy_from_slice(w.to_be_bytes().as_ref());
                buf[14..16].copy_from_slice(h.to_be_bytes().as_ref());
                buf[16..18].copy_from_slice(0u16.to_be_bytes().as_ref());
                buf[18..20].copy_from_slice(mx.to_be_bytes().as_ref());
                buf[20..22].copy_from_slice(my.to_be_bytes().as_ref());
                out.write_all(&buf).await?;
                Ok(())
            }
            Packet::ClientNoOp => {
                out.write_str("CNOP").await?;
                Ok(())
            }
            Packet::Unknown(_) => {
                unimplemented!()
            }
            Packet::InfoAck => {
                out.write_str("CIAK").await?;
                Ok(())
            }
            Packet::KeepAlive => {
                out.write_str("CALV").await?;
                Ok(())
            }
            Packet::ErrorUnknownDevice => {
                out.write_str("EUNK").await?;
                Ok(())
            }
            Packet::MouseMoveAbs { x, y } => {
                let mut buf = [0u8; 4 + 4 + 2 + 2];
                buf[0..4].copy_from_slice((4u32 + 2 + 2).to_be_bytes().as_ref());
                buf[4..8].copy_from_slice(b"DMMV");
                buf[8..10].copy_from_slice(x.to_be_bytes().as_ref());
                buf[10..12].copy_from_slice(y.to_be_bytes().as_ref());
                out.write_all(&buf).await?;
                Ok(())
            }
            _ => {
                unimplemented!("{:?} not yet implemented", self)
            }
        }
    }
}
