#[cfg(feature = "clipboard")]
use log::{debug, warn};
use tokio::io::{AsyncRead, AsyncReadExt};

#[cfg(feature = "clipboard")]
use crate::{clipboard::parse_clipboard, ClipboardStage};

use super::{Packet, PacketError, PacketReader, PacketWriter};

pub struct PacketStream<S: PacketReader + PacketWriter> {
    stream: S,
}

impl<S: PacketReader + PacketWriter> PacketStream<S> {
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub async fn read(
        &mut self,
        #[cfg(feature = "clipboard")] clipboard_stage: &mut ClipboardStage,
    ) -> Result<Packet, PacketError> {
        let size = self.stream.read_packet_size().await?;
        if size < 4 {
            let mut buf = [0; 4];
            self.stream.read_exact(&mut buf[0..size as usize]).await?;
            return Err(PacketError::PacketTooSmall);
        }
        Self::do_read(
            &mut self.stream,
            size as usize,
            #[cfg(feature = "clipboard")]
            clipboard_stage,
        )
        .await
    }

    async fn do_read<T: AsyncRead + Send + Unpin>(
        chunk: &mut T,
        mut limit: usize,
        #[cfg(feature = "clipboard")] clipboard_stage: &mut ClipboardStage,
    ) -> Result<Packet, PacketError> {
        let code: [u8; 4] = chunk.read_bytes_fixed().await?;
        limit -= 4;

        let packet = match code.as_ref() {
            b"QINF" => Packet::QueryInfo,
            b"CIAK" => Packet::InfoAck,
            b"CALV" => Packet::KeepAlive,
            #[cfg(feature = "barrier-options")]
            b"CROP" => Packet::ResetOptions,
            #[cfg(feature = "barrier-options")]
            b"DSOP" => {
                let num_items = chunk.read_u32().await?;
                limit -= 4;
                let num_opts = num_items / 2;
                let mut options: std::collections::HashMap<String, u32> =
                    std::collections::HashMap::new();
                // Currently only HBRT(Heartbeat interval) is supported
                for _ in 0..num_opts {
                    let opt: [u8; 4] = chunk.read_bytes_fixed().await?;
                    limit -= 4;
                    let val = chunk.read_u32().await?;
                    limit -= 4;
                    options.insert(String::from_utf8_lossy(&opt).into_owned(), val);
                }
                Packet::SetDeviceOptions(options)
            }
            b"EUNK" => Packet::ErrorUnknownDevice,
            b"DMMV" => {
                let x = chunk.read_u16().await?;
                limit -= 2;
                let y = chunk.read_u16().await?;
                limit -= 2;
                Packet::MouseMoveAbs { x, y }
            }
            b"DMRM" => {
                let x = chunk.read_i16().await?;
                limit -= 2;
                let y = chunk.read_i16().await?;
                limit -= 2;
                Packet::MouseMove { x, y }
            }
            b"CINN" => {
                let x = chunk.read_u16().await?;
                limit -= 2;
                let y = chunk.read_u16().await?;
                limit -= 2;
                let seq_num = chunk.read_u32().await?;
                limit -= 4;
                let mask = chunk.read_u16().await?;
                limit -= 2;
                Packet::CursorEnter {
                    x,
                    y,
                    seq_num,
                    mask,
                }
            }
            b"COUT" => Packet::CursorLeave,
            b"CCLP" => {
                let id = chunk.read_u8().await?;
                limit -= 1;
                let seq_num = chunk.read_u32().await?;
                limit -= 4;
                Packet::GrabClipboard { id, seq_num }
            }
            #[cfg(feature = "clipboard")]
            b"DCLP" => {
                let id = chunk.read_u8().await?;
                limit -= 1;
                let _seq_num = chunk.read_u32().await?;
                limit -= 4;
                let mark = chunk.read_u8().await?;
                limit -= 1;
                // chunk.read_to_end(&mut buf).await?;
                if limit > 0 {
                    let mut buf = Vec::with_capacity(limit);
                    chunk.read_exact(&mut buf).await?;
                }
                limit = 0;

                // mark 1 is the total length string in ASCII
                // mark 2 is the actual data and is split into chunks
                // mark 3 is an empty chunk
                debug!("Current Clipboard stage: {}", clipboard_stage.stage());
                *clipboard_stage = match mark {
                    1 => match clipboard_stage {
                        ClipboardStage::None => {
                            debug!("0 -> 1");
                            let _sz = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                            let expected_size = String::from_utf8_lossy(&buf[4..])
                                .parse::<u32>()
                                .map_err(|_| PacketError::FormatError)?;
                            debug!("Expected clipboard size: {}", expected_size);
                            ClipboardStage::Mark1 { id, data: vec![] }
                        }
                        ClipboardStage::Mark3 { id, .. } => {
                            debug!("3 -> 1");
                            ClipboardStage::Mark1 {
                                id: *id,
                                data: vec![],
                            }
                        }
                        _ => {
                            warn!(
                                "Unexpected clipboard stage transition from {} to 1",
                                clipboard_stage.stage()
                            );
                            ClipboardStage::None
                        }
                    },
                    2 => match clipboard_stage {
                        ClipboardStage::Mark1 { id, data } => {
                            debug!("1 -> 2");
                            ClipboardStage::Mark2 {
                                id: *id,
                                data: {
                                    data.extend_from_slice(&buf);
                                    data.to_vec()
                                },
                            }
                        }
                        ClipboardStage::Mark2 { id, data } => {
                            debug!("2 -> 2");

                            ClipboardStage::Mark2 {
                                id: *id,
                                data: {
                                    data.extend_from_slice(&buf);
                                    data.to_vec()
                                },
                            }
                        }
                        _ => {
                            warn!(
                                "Unexpected clipboard stage transition from {} to 2",
                                clipboard_stage.stage()
                            );
                            ClipboardStage::None
                        }
                    },
                    3 => match clipboard_stage {
                        ClipboardStage::Mark1 { id, data } => {
                            debug!("1 -> 3");
                            ClipboardStage::Mark3 {
                                id: *id,
                                data: {
                                    data.extend_from_slice(&buf);
                                    data.to_vec()
                                },
                            }
                        }
                        ClipboardStage::Mark2 { id, data } => {
                            debug!("2 -> 3");
                            ClipboardStage::Mark3 {
                                id: *id,
                                data: {
                                    data.extend_from_slice(&buf);
                                    data.to_vec()
                                },
                            }
                        }
                        _ => {
                            warn!(
                                "Unexpected clipboard stage transition from {} to 3",
                                clipboard_stage.stage()
                            );
                            ClipboardStage::None
                        }
                    },
                    _ => {
                        warn!("Unexpected clipboard mark: {}", mark);
                        ClipboardStage::None
                    }
                };
                match clipboard_stage {
                    ClipboardStage::Mark3 { id, data } => Packet::SetClipboard {
                        id: *id,
                        data: parse_clipboard(data).await?,
                    },
                    _ => Packet::ClientNoOp,
                }
            }

            b"DMUP" => {
                let id = chunk.read_i8().await?;
                limit -= 1;
                Packet::MouseUp { id }
            }
            b"DMDN" => {
                let id = chunk.read_i8().await?;
                limit -= 1;
                Packet::MouseDown { id }
            }
            b"DKUP" => {
                let id = chunk.read_u16().await?;
                limit -= 2;
                let mask = chunk.read_u16().await?;
                limit -= 2;
                let button = chunk.read_u16().await?;
                limit -= 2;
                Packet::KeyUp { id, mask, button }
            }
            b"DKDN" => {
                let id = chunk.read_u16().await?;
                limit -= 2;
                let mask = chunk.read_u16().await?;
                limit -= 2;
                let button = chunk.read_u16().await?;
                limit -= 2;
                Packet::KeyDown { id, mask, button }
            }
            b"DKRP" => {
                let id = chunk.read_u16().await?;
                limit -= 2;
                let mask = chunk.read_u16().await?;
                limit -= 2;
                let count = chunk.read_u16().await?;
                limit -= 2;
                let button = chunk.read_u16().await?;
                limit -= 2;
                Packet::KeyRepeat {
                    id,
                    mask,
                    button,
                    count,
                }
            }
            b"DMWM" => {
                let x_delta = chunk.read_i16().await?;
                limit -= 2;
                let y_delta = chunk.read_i16().await?;
                limit -= 2;
                Packet::MouseWheel { x_delta, y_delta }
            }
            _ => Packet::Unknown(code),
        };

        // Discard the rest of the packet
        chunk.discard_exact(limit).await?;

        Ok(packet)
    }

    pub async fn write(&mut self, packet: Packet) -> Result<(), PacketError> {
        packet.write_wire(&mut self.stream).await
    }
}
