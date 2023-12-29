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
    async fn connected(&mut self) -> Result<(), barrier_client::ActuatorError> {
        info!("Connected");
        Ok(())
    }

    async fn disconnected(&mut self) -> Result<(), barrier_client::ActuatorError> {
        info!("Disconnected");
        Ok(())
    }

    async fn get_screen_size(&self) -> Result<(u16, u16), barrier_client::ActuatorError> {
        Ok((self.width, self.height))
    }

    async fn get_cursor_position(&self) -> Result<(u16, u16), barrier_client::ActuatorError> {
        Ok((self.x, self.y))
    }

    async fn set_cursor_position(
        &mut self,
        x: u16,
        y: u16,
    ) -> Result<(), barrier_client::ActuatorError> {
        self.x = x;
        self.y = y;
        let report = &mut [0; 9];
        let ret = self.hid.set_cursor_position(x, y, report);
        info!("Set cursor position to {x} {y}, HID report: {:?}", ret);
        Ok(())
    }

    async fn move_cursor(&mut self, x: i16, y: i16) -> Result<(), barrier_client::ActuatorError> {
        self.x = (self.x as i32 + x as i32) as u16;
        self.y = (self.y as i32 + y as i32) as u16;
        self.set_cursor_position(self.x, self.y).await
    }

    async fn mouse_down(&mut self, button: i8) -> Result<(), barrier_client::ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_down(button, report);
        info!("Mouse button {button} down, HID report: {:?}", ret);
        Ok(())
    }

    async fn mouse_up(&mut self, button: i8) -> Result<(), barrier_client::ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_up(button, report);
        info!("Mouse button {button} up, HID report: {:?}", ret);
        Ok(())
    }

    async fn mouse_wheel(&mut self, x: i16, y: i16) -> Result<(), barrier_client::ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_scroll(x, y, report);
        info!("Mouse wheel {x} {y}, HID report: {:?}", ret);
        Ok(())
    }

    async fn key_down(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
    ) -> Result<(), barrier_client::ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.key_down(key, mask, button, report);
        info!("Key down {key} {mask} {button}, HID report: {:?}", ret);
        Ok(())
    }

    async fn key_repeat(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
        count: u16,
    ) -> Result<(), barrier_client::ActuatorError> {
        info!("Key repeat {key} {mask} {button} {count}");
        Ok(())
    }

    async fn key_up(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
    ) -> Result<(), barrier_client::ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.key_up(key, mask, button, report);
        info!("Key up {key} {mask} {button}, HID report: {:?}", ret);
        Ok(())
    }

    async fn enter(&mut self) -> Result<(), barrier_client::ActuatorError> {
        info!("Enter");
        Ok(())
    }

    async fn leave(&mut self) -> Result<(), barrier_client::ActuatorError> {
        info!("Leave");
        Ok(())
    }

    async fn set_options(
        &mut self,
        opts: std::collections::HashMap<String, u32>,
    ) -> Result<(), barrier_client::ActuatorError> {
        info!("Set options {:#?}", opts);
        Ok(())
    }

    async fn reset_options(&mut self) -> Result<(), barrier_client::ActuatorError> {
        info!("Reset options");
        Ok(())
    }

    async fn set_clipboard(
        &mut self,
        data: ClipboardData,
    ) -> Result<(), barrier_client::ActuatorError> {
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
        todo!()
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
