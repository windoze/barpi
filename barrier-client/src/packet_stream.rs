use std::collections::HashMap;

use async_trait::async_trait;
use log::{debug, warn};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::{client::ClipboardStage, clipboard::parse_clipboard};

use super::{Packet, PacketError, PacketReader, PacketWriter};

pub struct PacketStream<S: PacketReader + PacketWriter> {
    stream: Option<S>,
}

#[async_trait]
trait DiscardAll: AsyncRead + Send + Unpin {
    async fn discard_all(&mut self) -> Result<(), PacketError> {
        let mut buf = [0; 1024];
        while self.read(&mut buf).await? > 0 {}
        Ok(())
    }
}

impl<S: AsyncRead + Send + Unpin> DiscardAll for S {}

impl<S: PacketReader + PacketWriter> PacketStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream: Some(stream),
        }
    }

    pub async fn read(
        &mut self,
        clipboard_stage: &mut ClipboardStage,
    ) -> Result<Packet, PacketError> {
        let size = self.stream.as_mut().unwrap().read_packet_size().await?;
        if size < 4 {
            let mut vec = Vec::new();
            self.stream.as_mut().unwrap().read_to_end(&mut vec).await?;
            return Err(PacketError::PacketTooSmall);
        }
        let mut chunk = self.stream.take().unwrap().take(size as u64);
        match Self::do_read(&mut chunk, clipboard_stage, size).await {
            Ok(packet) => {
                self.stream = Some(chunk.into_inner());
                Ok(packet)
            }
            Err(e) => {
                self.stream = Some(chunk.into_inner());
                Err(e)
            }
        }
    }

    async fn do_read<T: AsyncRead + Send + Unpin>(
        chunk: &mut T,
        clipboard_stage: &mut ClipboardStage,
        size: u32,
    ) -> Result<Packet, PacketError> {
        let code: [u8; 4] = chunk.read_bytes_fixed().await?;
        // if size > 2048 {
        //     warn!("Packet too large, discarding {} bytes", size);
        //     chunk.discard_all().await?;
        //     return Ok(Packet::Unknown(code));
        // }

        let packet = match code.as_ref() {
            b"QINF" => Packet::QueryInfo,
            b"CIAK" => Packet::InfoAck,
            b"CALV" => Packet::KeepAlive,
            // We don't really have any option to set and reset
            // b"CROP" => Packet::ResetOptions,
            b"DSOP" => {
                let num_items = chunk.read_u32().await?;
                let num_opts = num_items / 2;
                let mut options: HashMap<String, u32> = HashMap::new();
                // Currently only HBRT(Heartbeat interval) is supported
                for _ in 0..num_opts {
                    let opt: [u8; 4] = chunk.read_bytes_fixed().await?;
                    let val = chunk.read_u32().await?;
                    options.insert(String::from_utf8_lossy(&opt).into_owned(), val);
                }
                Packet::SetDeviceOptions(options)
            }
            b"EUNK" => Packet::ErrorUnknownDevice,
            b"DMMV" => {
                let x = chunk.read_u16().await?;
                let y = chunk.read_u16().await?;
                Packet::MouseMoveAbs { x, y }
            }
            b"DMRM" => {
                let x = chunk.read_i16().await?;
                let y = chunk.read_i16().await?;
                Packet::MouseMove { x, y }
            }
            b"CINN" => {
                let x = chunk.read_u16().await?;
                let y = chunk.read_u16().await?;
                let seq_num = chunk.read_u32().await?;
                let mask = chunk.read_u16().await?;
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
                let seq_num = chunk.read_u32().await?;
                Packet::GrabClipboard { id, seq_num }
            }
            b"DCLP" => {
                let id = chunk.read_u8().await?;
                let _seq_num = chunk.read_u32().await?;
                let mark = chunk.read_u8().await?;
                let mut buf = vec![];
                chunk.read_to_end(&mut buf).await?;
                debug!("DCLP chunk, size: {}, mark: {}", size, mark);

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
                Packet::MouseUp { id }
            }
            b"DMDN" => {
                let id = chunk.read_i8().await?;
                Packet::MouseDown { id }
            }
            b"DKUP" => {
                let id = chunk.read_u16().await?;
                let mask = chunk.read_u16().await?;
                let button = chunk.read_u16().await?;
                Packet::KeyUp { id, mask, button }
            }
            b"DKDN" => {
                let id = chunk.read_u16().await?;
                let mask = chunk.read_u16().await?;
                let button = chunk.read_u16().await?;
                Packet::KeyDown { id, mask, button }
            }
            b"DKRP" => {
                let id = chunk.read_u16().await?;
                let mask = chunk.read_u16().await?;
                let count = chunk.read_u16().await?;
                let button = chunk.read_u16().await?;
                Packet::KeyRepeat {
                    id,
                    mask,
                    button,
                    count,
                }
            }
            b"DMWM" => {
                let x_delta = chunk.read_i16().await?;
                let y_delta = chunk.read_i16().await?;
                Packet::MouseWheel { x_delta, y_delta }
            }
            _ => Packet::Unknown(code),
        };

        // Discard the rest of the packet
        // warn!(
        //     "Discarding rest of packet, code: {}",
        //     String::from_utf8_lossy(code.as_ref())
        // );
        chunk.discard_all().await?;

        Ok(packet)
    }

    pub async fn write(&mut self, packet: Packet) -> Result<(), PacketError> {
        packet
            .write_wire(&mut self.stream.as_mut().unwrap())
            .await?;
        Ok(())
    }
}
