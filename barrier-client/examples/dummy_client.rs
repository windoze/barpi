use barrier_client::{self, start, Actuator};
use env_logger::Env;
use log::info;

#[cfg(feature = "clipboard")]
use barrier_client::ClipboardData;

struct DummyActuator {
    width: u16,
    height: u16,
    x: u16,
    y: u16,
    #[cfg(feature = "barrier-options")]
    options: std::collections::HashMap<String, u32>,
}

impl Actuator for DummyActuator {
    fn connected(&mut self) {
        info!("Connected");
    }

    fn disconnected(&mut self) {
        info!("Disconnected");
    }

    fn get_screen_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    fn get_cursor_position(&self) -> (u16, u16) {
        (self.x, self.y)
    }

    fn set_cursor_position(&mut self, x: u16, y: u16) {
        self.x = x;
        self.y = y;
        info!("Set cursor position to {x} {y}");
    }

    fn move_cursor(&mut self, x: i16, y: i16) {
        self.x = (self.x as i32 + x as i32) as u16;
        self.y = (self.y as i32 + y as i32) as u16;
        info!("Move cursor by {x} {y}, now at {} {}", self.x, self.y);
    }

    fn mouse_down(&mut self, button: i8) {
        info!("Mouse down {button}");
    }

    fn mouse_up(&mut self, button: i8) {
        info!("Mouse up {button}");
    }

    fn mouse_wheel(&mut self, x: i16, y: i16) {
        info!("Mouse wheel {x} {y}")
    }

    fn key_down(&mut self, key: u16, mask: u16, button: u16) {
        info!("Key down {key} {mask} {button}")
    }

    fn key_repeat(&mut self, key: u16, mask: u16, button: u16, count: u16) {
        info!("Key repeat {key} {mask} {button} {count}")
    }

    fn key_up(&mut self, key: u16, mask: u16, button: u16) {
        info!("Key up {key} {mask} {button}")
    }

    #[cfg(feature = "barrier-options")]
    fn set_options(&mut self, opts: std::collections::HashMap<String, u32>) {
        self.options = opts;
        info!("Set options {:#?}", self.options)
    }

    #[cfg(feature = "barrier-options")]
    fn reset_options(&mut self) {
        self.options.clear();
        info!("Reset options")
    }

    fn enter(&mut self) {
        info!("Enter")
    }

    fn leave(&mut self) {
        info!("Leave")
    }

    #[cfg(feature = "clipboard")]
    fn set_clipboard(&mut self, data: ClipboardData) {
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
        #[cfg(feature = "barrier-options")]
        options: std::collections::HashMap::new(),
    };
    start("192.168.2.59:24800", String::from("BARPI"), &mut actuator)
        .await
        .unwrap();
}
