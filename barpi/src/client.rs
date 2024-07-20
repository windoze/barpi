use std::{fs::File, io::Write};

use barrier_client::{Actuator, ActuatorError, ClipboardData};
use log::{debug, error, info};
use synergy_hid::{ReportType, SynergyHid};
use tokio_util::sync::CancellationToken;
pub struct BarpiActuator {
    width: u16,
    height: u16,
    x: u16,
    y: u16,
    hid: SynergyHid,
    keyboard_file: File,
    mouse_file: File,
    consumer_file: File,
    token: CancellationToken,
}

impl BarpiActuator {
    pub fn new(
        width: u16,
        height: u16,
        flip_mouse_wheel: bool,
        keyboard_file: File,
        mouse_file: File,
        consumer_file: File,
        token: CancellationToken,
    ) -> Self {
        Self {
            width,
            height,
            x: 0,
            y: 0,
            hid: SynergyHid::new(flip_mouse_wheel),
            keyboard_file,
            mouse_file,
            consumer_file,
            token,
        }
    }

    pub(crate) fn scale_position(&self, x: u16, y: u16) -> (u16, u16) {
        (
            ((x as f32) * (self.width as f32) / 0x7fff as f32).ceil() as u16,
            ((y as f32) * (self.height as f32) / 0x7fff as f32).ceil() as u16,
        )
    }

    fn write_report(&mut self, report: (ReportType, &[u8])) {
        let r = match report.0 {
            ReportType::Keyboard => self.keyboard_file.write_all(report.1),
            ReportType::Mouse => self.mouse_file.write_all(report.1),
            ReportType::Consumer => self.consumer_file.write_all(report.1),
            ReportType::Status => todo!(),
        };
        match r {
            Ok(_) => (),
            Err(e) => {
                error!("Error writing report: {:?}", e);
                self.token.cancel();
            }
        }
    }
}

impl Actuator for BarpiActuator {
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
        (self.x, self.y) = self.scale_position(x, y);
        let report = &mut [0; 9];
        let ret = self.hid.set_cursor_position(x, y, report);
        debug!("Set cursor position to {x} {y}, HID report: {:?}", ret);
        self.write_report(ret);
        Ok(())
    }

    async fn move_cursor(&mut self, x: i16, y: i16) -> Result<(), ActuatorError> {
        self.x = (self.x as i32 + x as i32) as u16;
        self.y = (self.y as i32 + y as i32) as u16;
        self.set_cursor_position(self.x, self.y).await
    }

    async fn mouse_down(&mut self, button: i8) -> Result<(), ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_down(button, report);
        debug!("Mouse button {button} down, HID report: {:?}", ret);
        self.write_report(ret);
        Ok(())
    }

    async fn mouse_up(&mut self, button: i8) -> Result<(), ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_up(button, report);
        debug!("Mouse button {button} up, HID report: {:?}", ret);
        self.write_report(ret);
        Ok(())
    }

    async fn mouse_wheel(&mut self, x: i16, y: i16) -> Result<(), ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_scroll(x, y, report);
        debug!("Mouse wheel {x} {y}, HID report: {:?}", ret);
        self.write_report(ret);
        Ok(())
    }

    async fn key_down(&mut self, key: u16, mask: u16, button: u16) -> Result<(), ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.key_down(key, mask, button, report);
        debug!("Key down {key} {mask} {button}, HID report: {:?}", ret);
        self.write_report(ret);
        Ok(())
    }

    async fn key_repeat(
        &mut self,
        key: u16,
        mask: u16,
        button: u16,
        count: u16,
    ) -> Result<(), ActuatorError> {
        debug!("Key repeat {key} {mask} {button} {count}");
        Ok(())
    }

    async fn key_up(&mut self, key: u16, mask: u16, button: u16) -> Result<(), ActuatorError> {
        let report = &mut [0; 9];
        let ret = self.hid.key_up(key, mask, button, report);
        debug!("Key up {key} {mask} {button}, HID report: {:?}", ret);
        self.write_report(ret);
        Ok(())
    }

    async fn enter(&mut self) -> Result<(), ActuatorError> {
        info!("Enter");
        Ok(())
    }

    async fn leave(&mut self) -> Result<(), ActuatorError> {
        info!("Leave");
        debug!("Clear HID reports");
        let report = &mut [0; 9];
        let ret = self.hid.clear(ReportType::Keyboard, report);
        self.write_report(ret);
        let ret = self.hid.clear(ReportType::Mouse, report);
        self.write_report(ret);
        let ret = self.hid.clear(ReportType::Consumer, report);

        self.write_report(ret);
        Ok(())
    }

    async fn set_options(
        &mut self,
        opts: std::collections::HashMap<String, u32>,
    ) -> Result<(), ActuatorError> {
        debug!("Set options {:#?}", opts);
        Ok(())
    }

    async fn reset_options(&mut self) -> Result<(), ActuatorError> {
        debug!("Reset options");
        Ok(())
    }

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

    async fn get_clipboard(&mut self) -> Result<Option<ClipboardData>, ActuatorError> {
        info!("Get clipboard");
        Ok(None)
    }
}
