use barrier_client::{self, start, Actuator, ClipboardData};
use env_logger::Env;
use log::info;

use synergy_hid::SynergyHid;

struct DummyActuator {
    width: u16,
    height: u16,
    x: u16,
    y: u16,
    hid: SynergyHid,
}

impl Actuator for DummyActuator {
    async fn connected(&mut self) {
        info!("Connected");
    }

    async fn disconnected(&mut self) {
        info!("Disconnected");
    }

    async fn get_screen_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    async fn get_cursor_position(&self) -> (u16, u16) {
        (self.x, self.y)
    }

    async fn set_cursor_position(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
        let report = &mut [0; 9];
        let ret = self.hid.set_cursor_position(x, y, report);
        info!("Set cursor position to {x} {y}, HID report: {:?}", ret);
    }

    async fn move_cursor(&mut self, x: i16, y: i16) {
        self.x = (self.x as i32 + x as i32) as u16;
        self.y = (self.y as i32 + y as i32) as u16;
        self.set_cursor_position(self.x, self.y).await;
    }

    async fn mouse_down(&mut self, button: i8) {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_down(button, report);
        info!("Mouse button {button} down, HID report: {:?}", ret);
    }

    async fn mouse_up(&mut self, button: i8) {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_up(button, report);
        info!("Mouse button {button} up, HID report: {:?}", ret);
    }

    async fn mouse_wheel(&mut self, x: i16, y: i16) {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_scroll(x, y, report);
        info!("Mouse wheel {x} {y}, HID report: {:?}", ret);
    }

    async fn key_down(&mut self, key: u16, mask: u16, button: u16) {
        let report = &mut [0; 9];
        let ret = self.hid.key_down(key, mask, button, report);
        info!("Key down {key} {mask} {button}, HID report: {:?}", ret);
    }

    async fn key_repeat(&mut self, key: u16, mask: u16, button: u16, count: u16) {
        info!("Key repeat {key} {mask} {button} {count}")
    }

    async fn key_up(&mut self, key: u16, mask: u16, button: u16) {
        let report = &mut [0; 9];
        let ret = self.hid.key_up(key, mask, button, report);
        info!("Key up {key} {mask} {button}, HID report: {:?}", ret);
    }

    async fn enter(&mut self) {
        info!("Enter")
    }

    async fn leave(&mut self) {
        info!("Leave")
    }

    async fn set_options(&mut self, opts: std::collections::HashMap<String, u32>) {
        info!("Set options {:#?}", opts)
    }

    async fn reset_options(&mut self) {
        info!("Reset options")
    }

    async fn set_clipboard(&mut self, data: ClipboardData) {
        info!(
            "Clipboard text:{}",
            data.text()
                .map(|s| s.as_str().chars().take(20).collect::<String>() + "...")
                .unwrap_or(String::from("<None>"))
        );
        info!(
            "Clipboard html:{}",
            data.html()
                .map(|s| s.as_str().chars().take(20).collect::<String>() + "...")
                .unwrap_or(String::from("<None>")),
        );
        info!(
            "Clipboard bitmap:{}",
            data.bitmap().map(|_| "yes").unwrap_or("no")
        );
    }
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    let mut actuator = DummyActuator {
        width: 1920,
        height: 1080,
        x: 0,
        y: 0,
        hid: SynergyHid::new(false),
    };
    start("192.168.2.59:24800", String::from("BARPI"), &mut actuator)
        .await
        .unwrap();
}
