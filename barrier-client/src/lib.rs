mod actuator;
mod client;
mod error;
mod packet;
mod packet_io;
mod packet_stream;

pub(crate) use error::{ConnectionError, PacketError};
pub(crate) use packet::Packet;
pub(crate) use packet_io::{PacketReader, PacketWriter};
pub(crate) use packet_stream::PacketStream;

pub use actuator::{Actuator, ActuatorMessage};
pub use client::start;
#[cfg(feature = "async-actuator")]
pub use actuator::AsyncActuator;
#[cfg(feature = "async-actuator")]
pub use client::start_async;

#[cfg(feature = "clipboard")]
mod clipboard;
#[cfg(feature = "clipboard")]
pub use clipboard::ClipboardData;
#[cfg(feature = "clipboard")]
pub(crate) use clipboard::ClipboardStage;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
