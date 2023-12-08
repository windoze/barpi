use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg(feature = "clipboard")]
use crate::ClipboardData;

pub trait Actuator {
    fn connected(&mut self);

    fn disconnected(&mut self);

    fn get_screen_size(&self) -> (u16, u16);

    fn get_cursor_position(&self) -> (u16, u16);

    fn set_cursor_position(&mut self, x: u16, y: u16);

    fn move_cursor(&mut self, x: i16, y: i16) {
        let (cx, cy) = self.get_cursor_position();
        self.set_cursor_position((cx as i32 + x as i32) as u16, (cy as i32 + y as i32) as u16);
    }

    fn mouse_down(&mut self, button: i8);

    fn mouse_up(&mut self, button: i8);

    fn mouse_wheel(&mut self, x: i16, y: i16);

    fn key_down(&mut self, key: u16, mask: u16, button: u16);

    fn key_repeat(&mut self, key: u16, mask: u16, button: u16, count: u16);

    fn key_up(&mut self, key: u16, mask: u16, button: u16);

    fn set_options(&mut self, opts: HashMap<String, u32>);

    fn reset_options(&mut self);

    fn enter(&mut self);

    fn leave(&mut self);

    #[cfg(feature = "clipboard")]
    fn set_clipboard(&mut self, data: ClipboardData);
}

#[cfg(feature = "async-actuator")]
use async_trait::async_trait;

#[cfg(feature = "async-actuator")]
#[async_trait]
pub trait AsyncActuator {
    async fn connected(&mut self);

    async fn disconnected(&mut self);

    async fn get_screen_size(&self) -> (u16, u16);

    async fn get_cursor_position(&self) -> (u16, u16);

    async fn set_cursor_position(&mut self, x: u16, y: u16);

    async fn move_cursor(&mut self, x: i16, y: i16) {
        let (cx, cy) = self.get_cursor_position().await;
        self.set_cursor_position((cx as i32 + x as i32) as u16, (cy as i32 + y as i32) as u16)
            .await;
    }

    async fn mouse_down(&mut self, button: i8);

    async fn mouse_up(&mut self, button: i8);

    async fn mouse_wheel(&mut self, x: i16, y: i16);

    async fn key_down(&mut self, key: u16, mask: u16, button: u16);

    async fn key_repeat(&mut self, key: u16, mask: u16, button: u16, count: u16);

    async fn key_up(&mut self, key: u16, mask: u16, button: u16);

    async fn set_options(&mut self, opts: HashMap<String, u32>);

    async fn reset_options(&mut self);

    async fn enter(&mut self);

    async fn leave(&mut self);

    #[cfg(feature = "clipboard")]
    async fn set_clipboard(&mut self, data: ClipboardData);
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
    SetOptions {
        opts: HashMap<String, u32>,
    },
    ResetOptions,
    Enter,
    Leave,
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
