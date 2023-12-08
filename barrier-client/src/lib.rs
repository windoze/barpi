mod actuator;
mod client;
mod clipboard;
mod error;
mod packet;
mod packet_io;
mod packet_stream;

pub(crate) use error::{ConnectionError, PacketError};
pub(crate) use packet::Packet;
pub(crate) use packet_io::{PacketReader, PacketWriter};
pub(crate) use packet_stream::PacketStream;

pub use actuator::{Actuator, ActuatorMessage, AsyncActuator};
pub use client::{start, start_async};
pub use clipboard::ClipboardData;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
