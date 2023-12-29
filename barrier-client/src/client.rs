use log::{debug, error, info};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

#[cfg(feature = "async-actuator")]
use crate::actuator::AsyncActuator;

use super::{Actuator, ConnectionError, Packet, PacketReader, PacketStream, PacketWriter};

pub async fn start<A: Actuator, Addr: ToSocketAddrs, S: AsRef<str>>(
    addr: Addr,
    device_name: S,
    actor: &mut A,
) -> Result<(), ConnectionError> {
    let screen_size: (u16, u16) = actor.get_screen_size().await?;

    let mut stream = TcpStream::connect(addr).await?;
    // Turn off Nagle, this may not be available on ESP-IDF, so ignore the error.
    stream.set_nodelay(true).ok();

    let _size = stream.read_packet_size().await?;
    if stream.read_bytes_fixed::<7>().await? == [b'B', b'a', b'r', b'r', b'i', b'e', b'r'] {
        debug!("Got hello");
    } else {
        error!("Got invalid hello");
        return Err(ConnectionError::ProtocolError(
            crate::error::PacketError::FormatError,
        ));
    }
    let major = stream.read_u16().await?;
    let minor = stream.read_u16().await?;
    debug!("Got hello {major}:{minor}");

    stream
        .write_u32("Barrier".len() as u32 + 2 + 2 + 4 + device_name.as_ref().bytes().len() as u32)
        .await?;
    stream.write_all(b"Barrier").await?;
    stream.write_u16(1).await?;
    stream.write_u16(6).await?;
    stream.write_str(device_name.as_ref()).await?;

    actor.connected().await?;

    let mut last_seq_num: u32 = 0;

    #[cfg(feature = "clipboard")]
    let mut clipboard_stage = crate::ClipboardStage::None;

    let mut packet_stream = PacketStream::new(stream);
    while let Ok(packet) = packet_stream
        .read(
            #[cfg(feature = "clipboard")]
            &mut clipboard_stage,
        )
        .await
    {
        match packet {
            Packet::QueryInfo => {
                match packet_stream
                    .write(Packet::DeviceInfo {
                        x: 0,
                        y: 0,
                        w: screen_size.0,
                        h: screen_size.1,
                        _dummy: 0,
                        mx: 0,
                        my: 0,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        actor.disconnected().await?;
                        Err(e)
                    }
                }?;
            }
            Packet::KeepAlive => {
                match packet_stream.write(Packet::KeepAlive).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        actor.disconnected().await?;
                        Err(e)
                    }
                }?;
            }
            Packet::MouseMoveAbs { x, y } => {
                let abs_x = ((x as f32) * (0x7fff as f32 / (screen_size.0 as f32))).ceil() as u16;
                let abs_y = ((y as f32) * (0x7fff as f32 / (screen_size.1 as f32))).ceil() as u16;
                actor.set_cursor_position(abs_x, abs_y).await?;
            }
            Packet::MouseMove { x, y } => {
                actor.move_cursor(x, y).await?;
            }
            Packet::KeyUp { id, mask, button } => {
                actor.key_up(id, mask, button).await?;
            }
            Packet::KeyDown { id, mask, button } => {
                actor.key_down(id, mask, button).await?;
            }
            Packet::KeyRepeat {
                id,
                mask,
                button,
                count,
            } => {
                actor.key_repeat(id, mask, button, count).await?;
            }
            Packet::MouseDown { id } => {
                actor.mouse_down(id).await?;
            }
            Packet::MouseUp { id } => {
                actor.mouse_up(id).await?;
            }
            Packet::MouseWheel { x_delta, y_delta } => {
                actor.mouse_wheel(x_delta, y_delta).await?;
            }
            Packet::InfoAck => { //Ignore
            }
            #[cfg(feature = "barrier-options")]
            Packet::ResetOptions => {
                actor.reset_options().await?;
            }
            #[cfg(feature = "barrier-options")]
            Packet::SetDeviceOptions(opts) => {
                actor.set_options(opts).await?;
            }
            Packet::CursorEnter { seq_num, .. } => {
                last_seq_num = seq_num;
                info!("Cursor enter: seq_num:{last_seq_num}");
                actor.enter().await?;
            }
            Packet::CursorLeave => {
                match actor.get_clipboard().await? {
                    #[cfg(feature = "clipboard")]
                    Some(data) => {
                        last_seq_num += 1;
                        info!("Clipboard: last_seq_num:{last_seq_num}, seq_num:{last_seq_num}, data:...");
                        packet_stream
                            .write(Packet::SetClipboard {
                                id: 0,
                                seq_num: last_seq_num,
                                data,
                            })
                            .await?;
                    }
                    None => {}
                }
                actor.leave().await?;
            }
            Packet::GrabClipboard { id, seq_num } => {
                info!("Grab clipboard: id:{id}, seq_num:{seq_num}");
            }
            #[cfg(feature = "clipboard")]
            Packet::SetClipboard { id, seq_num, data } => {
                if !data.is_empty() {
                    // last_seq_num = seq_num;
                    info!("Clipboard: id:{id}, last_seq_num:{last_seq_num}, seq_num:{seq_num}, data:...");
                    actor.set_clipboard(data).await?;
                }
            }
            Packet::DeviceInfo { .. } | Packet::ErrorUnknownDevice | Packet::ClientNoOp => {
                // Server only packets
            }
            Packet::Unknown(cmd) => {
                debug!(
                    "Unknown packet: {}",
                    core::str::from_utf8(&cmd).unwrap_or("????")
                );
            }
        }
    }
    actor.disconnected().await?;
    Err(ConnectionError::Disconnected)
}
