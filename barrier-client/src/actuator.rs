use serde::{Deserialize, Serialize};

use crate::error::ActuatorError;
#[cfg(feature = "clipboard")]
use crate::ClipboardData;

pub trait Actuator {
    fn connected(&mut self) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn disconnected(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn get_screen_size(
        &self,
    ) -> impl std::future::Future<Output = Result<(u16, u16), ActuatorError>> + Send;

    fn get_cursor_position(
        &self,
    ) -> impl std::future::Future<Output = Result<(u16, u16), ActuatorError>> + Send;

    fn set_cursor_position(
        &mut self,
        x: u16,
        y: u16,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn move_cursor(
        &mut self,
        x: i16,
        y: i16,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn mouse_down(
        &mut self,
        button: i8,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn mouse_up(
        &mut self,
        button: i8,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn mouse_wheel(
        &mut self,
        x: i16,
        y: i16,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn key_down(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn key_repeat(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
        count: u16,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn key_up(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    #[cfg(feature = "barrier-options")]
    fn set_options(
        &mut self,
        opts: std::collections::HashMap<String, u32>,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    #[cfg(feature = "barrier-options")]
    fn reset_options(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn enter(&mut self) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    fn leave(&mut self) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;

    #[cfg(feature = "clipboard")]
    fn get_clipboard(
        &mut self,
    ) -> impl std::future::Future<Output = Result<Option<ClipboardData>, ActuatorError>> + Send;

    #[cfg(feature = "clipboard")]
    fn set_clipboard(
        &mut self,
        data: ClipboardData,
    ) -> impl std::future::Future<Output = Result<(), ActuatorError>> + Send;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ActuatorMessage {
    Connected,
    Disconnected,
    SetCursorPosition {
        x: u16,
        y: u16,
    },
    MoveCursor {
        x: i16,
        y: i16,
    },
    MouseDown {
        button: i8,
    },
    MouseUp {
        button: i8,
    },
    MouseWheel {
        x: i16,
        y: i16,
    },
    KeyDown {
        key: u16,
        mask: u16,
        button: u16,
    },
    KeyRepeat {
        key: u16,
        mask: u16,
        button: u16,
        count: u16,
    },
    KeyUp {
        key: u16,
        mask: u16,
        button: u16,
    },
    Enter,
    Leave,
    #[cfg(feature = "barrier-options")]
    SetOptions {
        opts: std::collections::HashMap<String, u32>,
    },
    #[cfg(feature = "barrier-options")]
    ResetOptions,
    #[cfg(feature = "clipboard")]
    SetClipboardText {
        data: String,
    },
    #[cfg(feature = "clipboard")]
    SetClipboardHtml {
        data: String,
    },
    #[cfg(feature = "clipboard")]
    SetClipboardBitmap {
        data: Vec<u8>,
    },
}
