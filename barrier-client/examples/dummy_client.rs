use barrier_client::{self, start, Actuator, ActuatorError};
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
    async fn connected(&mut self) -> Result<(), ActuatorError> {
        info!("Connected");
        Ok(())
    }

    async fn disconnected(&mut self) -> Result<(), ActuatorError> {
        info!("Disconnected");
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u16, u16), ActuatorError> {
        Ok((self.width, self.height))
    }

    async fn get_cursor_position(&self) -> Result<(u16, u16), ActuatorError> {
        Ok((self.x, self.y))
    }

    async fn set_cursor_position(&mut self, x: u16, y: u16) -> Result<(), ActuatorError> {
        self.x = x;
        self.y = y;
        info!("Set cursor position to {x} {y}");
        Ok(())
    }

    async fn move_cursor(&mut self, x: i16, y: i16) -> Result<(), ActuatorError> {
        self.x = (self.x as i32 + x as i32) as u16;
        self.y = (self.y as i32 + y as i32) as u16;
        info!("Move cursor by {x} {y}, now at {} {}", self.x, self.y);
        Ok(())
    }

    async fn mouse_down(&mut self, button: i8) -> Result<(), ActuatorError> {
        info!("Mouse down {button}");
        Ok(())
    }

    async fn mouse_up(&mut self, button: i8) -> Result<(), ActuatorError> {
        info!("Mouse up {button}");
        Ok(())
    }

    async fn mouse_wheel(&mut self, x: i16, y: i16) -> Result<(), ActuatorError> {
        info!("Mouse wheel {x} {y}");
        Ok(())
    }

    async fn key_down(&mut self, key: u16, mask: u16, button: u16) -> Result<(), ActuatorError> {
        info!("Key down {key} {mask} {button}");
        Ok(())
    }

    async fn key_repeat(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
        count: u16,
    ) -> Result<(), ActuatorError> {
        info!("Key repeat {key} {mask} {button} {count}");
        Ok(())
    }

    async fn key_up(&mut self, key: u16, mask: u16, button: u16) -> Result<(), ActuatorError> {
        info!("Key up {key} {mask} {button}");
        Ok(())
    }

    #[cfg(feature = "barrier-options")]
    async fn set_options(
        &mut self,
        opts: std::collections::HashMap<String, u32>,
    ) -> Result<(), ActuatorError> {
        self.options = opts;
        info!("Set options {:#?}", self.options);
        Ok(())
    }

    #[cfg(feature = "barrier-options")]
    async fn reset_options(&mut self) -> Result<(), ActuatorError> {
        self.options.clear();
        info!("Reset options");
        Ok(())
    }

    async fn enter(&mut self) -> Result<(), ActuatorError> {
        info!("Enter");
        Ok(())
    }

    async fn leave(&mut self) -> Result<(), ActuatorError> {
        info!("Leave");
        Ok(())
    }

    #[cfg(feature = "clipboard")]
    async fn set_clipboard(&mut self, data: ClipboardData) -> Result<(), ActuatorError> {
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
        Ok(())
    }

    async fn get_clipboard(
        &mut self,
    ) -> Result<Option<ClipboardData>, barrier_client::ActuatorError> {
        info!("Get clipboard");
        Ok(None)
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
