use log::debug;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, ToSocketAddrs},
};

use crate::actuator::AsyncActuator;

use super::{Actuator, ConnectionError, Packet, PacketReader, PacketStream, PacketWriter};

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

pub async fn start<A: Actuator, Addr: ToSocketAddrs>(
    addr: Addr,
    device_name: String,
    actor: &mut A,
) -> Result<(), ConnectionError> {
    let screen_size: (u16, u16) = actor.get_screen_size();

    let mut stream = TcpStream::connect(addr).await?;
    // Turn off Nagle, this may not be available on ESP-IDF, so ignore the error.
    stream.set_nodelay(true).ok();

    let _size = stream.read_packet_size().await?;
    stream.read_str_lit("Barrier").await?;
    let major = stream.read_u16().await?;
    let minor = stream.read_u16().await?;
    debug!("Got hello {major}:{minor}");

    stream
        .write_u32("Barrier".len() as u32 + 2 + 2 + 4 + device_name.bytes().len() as u32)
        .await?;
    stream.write_all(b"Barrier").await?;
    stream.write_u16(1).await?;
    stream.write_u16(6).await?;
    stream.write_str(&device_name).await?;

    actor.connected();

    let mut clipboard_stage = ClipboardStage::None;
    let mut packet_stream = PacketStream::new(stream);
    while let Ok(packet) = packet_stream.read(&mut clipboard_stage).await {
        match packet {
            Packet::QueryInfo => {
                packet_stream
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
                    .map_err(|e| {
                        actor.disconnected();
                        e
                    })?;
            }
            Packet::KeepAlive => {
                packet_stream.write(Packet::KeepAlive).await.map_err(|e| {
                    actor.disconnected();
                    e
                })?;
            }
            Packet::MouseMoveAbs { x, y } => {
                let abs_x = ((x as f32) * (0x7fff as f32 / (screen_size.0 as f32))).ceil() as u16;
                let abs_y = ((y as f32) * (0x7fff as f32 / (screen_size.1 as f32))).ceil() as u16;
                actor.set_cursor_position(abs_x, abs_y);
            }
            Packet::MouseMove { x, y } => {
                actor.move_cursor(x, y);
            }
            Packet::KeyUp { id, mask, button } => {
                actor.key_up(id, mask, button);
            }
            Packet::KeyDown { id, mask, button } => {
                actor.key_down(id, mask, button);
            }
            Packet::KeyRepeat {
                id,
                mask,
                button,
                count,
            } => {
                actor.key_repeat(id, mask, button, count);
            }
            Packet::MouseDown { id } => {
                actor.mouse_down(id);
            }
            Packet::MouseUp { id } => {
                actor.mouse_up(id);
            }
            Packet::MouseWheel { x_delta, y_delta } => {
                actor.mouse_wheel(x_delta, y_delta);
            }
            Packet::InfoAck => { //Ignore
            }
            Packet::ResetOptions => {
                actor.reset_options();
            }
            Packet::SetDeviceOptions(opts) => {
                actor.set_options(opts);
            }
            Packet::CursorEnter { .. } => {
                actor.enter();
            }
            Packet::CursorLeave => {
                actor.leave();
            }
            Packet::GrabClipboard { .. } => {}
            Packet::SetClipboard { id, data } => {
                if !data.is_empty() {
                    debug!("Clipboard: id:{id}, data:...");
                    actor.set_clipboard(data);
                }
            }
            Packet::DeviceInfo { .. } | Packet::ErrorUnknownDevice | Packet::ClientNoOp => {
                // Server only packets
            }
            Packet::Unknown(cmd) => {
                debug!("Unknown packet: {:?}", cmd);
            }
        }
    }
    actor.disconnected();
    Err(ConnectionError::Disconnected)
}

pub async fn start_async<A: AsyncActuator + Send + Unpin, Addr: ToSocketAddrs>(
    addr: Addr,
    device_name: String,
    actor: &mut A,
) -> Result<(), ConnectionError> {
    let screen_size: (u16, u16) = actor.get_screen_size().await;

    let mut stream = TcpStream::connect(addr).await?;
    // Turn off Nagle, this may not be available on ESP-IDF, so ignore the error.
    stream.set_nodelay(true).ok();

    let _size = stream.read_packet_size().await?;
    stream.read_str_lit("Barrier").await?;
    let major = stream.read_u16().await?;
    let minor = stream.read_u16().await?;
    debug!("Got hello {major}:{minor}");

    stream
        .write_u32("Barrier".len() as u32 + 2 + 2 + 4 + device_name.bytes().len() as u32)
        .await?;
    stream.write_all(b"Barrier").await?;
    stream.write_u16(1).await?;
    stream.write_u16(6).await?;
    stream.write_str(&device_name).await?;

    actor.connected().await;

    let mut clipboard_stage = ClipboardStage::None;
    let mut packet_stream = PacketStream::new(stream);
    while let Ok(packet) = packet_stream.read(&mut clipboard_stage).await {
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
                        actor.disconnected().await;
                        Err(e)
                    }
                }?;
            }
            Packet::KeepAlive => {
                match packet_stream.write(Packet::KeepAlive).await {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        actor.disconnected().await;
                        Err(e)
                    }
                }?;
            }
            Packet::MouseMoveAbs { x, y } => {
                let abs_x = ((x as f32) * (0x7fff as f32 / (screen_size.0 as f32))).ceil() as u16;
                let abs_y = ((y as f32) * (0x7fff as f32 / (screen_size.1 as f32))).ceil() as u16;
                actor.set_cursor_position(abs_x, abs_y).await;
            }
            Packet::MouseMove { x, y } => {
                actor.move_cursor(x, y).await;
            }
            Packet::KeyUp { id, mask, button } => {
                actor.key_up(id, mask, button).await;
            }
            Packet::KeyDown { id, mask, button } => {
                actor.key_down(id, mask, button).await;
            }
            Packet::KeyRepeat {
                id,
                mask,
                button,
                count,
            } => {
                actor.key_repeat(id, mask, button, count).await;
            }
            Packet::MouseDown { id } => {
                actor.mouse_down(id).await;
            }
            Packet::MouseUp { id } => {
                actor.mouse_up(id).await;
            }
            Packet::MouseWheel { x_delta, y_delta } => {
                actor.mouse_wheel(x_delta, y_delta).await;
            }
            Packet::InfoAck => { //Ignore
            }
            Packet::ResetOptions => {
                actor.reset_options().await;
            }
            Packet::SetDeviceOptions(opts) => {
                actor.set_options(opts).await;
            }
            Packet::CursorEnter { .. } => {
                actor.enter().await;
            }
            Packet::CursorLeave => {
                actor.leave().await;
            }
            Packet::GrabClipboard { .. } => {}
            Packet::SetClipboard { id, data } => {
                if !data.is_empty() {
                    debug!("Clipboard: id:{id}, data:...");
                    actor.set_clipboard(data).await;
                }
            }
            Packet::DeviceInfo { .. } | Packet::ErrorUnknownDevice | Packet::ClientNoOp => {
                // Server only packets
            }
            Packet::Unknown(cmd) => {
                debug!("Unknown packet: {:?}", cmd);
            }
        }
    }
    actor.disconnected().await;
    Err(ConnectionError::Disconnected)
}
