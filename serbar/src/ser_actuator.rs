use barrier_client::{Actuator, ClipboardData};
use log::{debug, info};
use synergy_hid::{ReportType, SynergyHid};
use tokio::io::AsyncWriteExt;
use tokio_serial::SerialStream;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicatorStatus {
    PowerOn,
    WifiConnecting,
    WifiConnected,
    ServerConnecting,
    ServerConnected,
    EnterScreen,
    LeaveScreen,
    ServerDisconnected,
}

pub struct SerbarActuator {
    width: u16,
    height: u16,
    x: u16,
    y: u16,
    hid: SynergyHid,
    port: SerialStream,
}

impl SerbarActuator {
    pub fn new(width: u16, height: u16, flip_mouse_wheel: bool, port: SerialStream) -> Self {
        Self {
            width,
            height,
            x: 0,
            y: 0,
            hid: SynergyHid::new(flip_mouse_wheel),
            port,
        }
    }

    async fn send_report(&mut self, report: &(ReportType, &[u8])) -> anyhow::Result<()> {
        let buf = &mut [0; 9];
        match report.0 {
            ReportType::Status => {
                buf[0] = 0;
                buf[1] = report.1[0];
            }
            ReportType::Keyboard => {
                buf[0] = 1;
                buf[1..9].copy_from_slice(&report.1[0..8]);
            }
            ReportType::Mouse => {
                buf[0] = 2;
                buf[1..8].copy_from_slice(&report.1[0..7]);
            }
            ReportType::Consumer => {
                buf[0] = 3;
                buf[1..3].copy_from_slice(&report.1[0..2]);
            }
        }
        self.port.write_all(buf).await?;
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        Ok(())
    }

    pub async fn clear(&mut self) -> anyhow::Result<()> {
        let report = &mut [0; 9];
        let ret = self.hid.clear(ReportType::Keyboard, report);
        info!("Clear keyboard, HID report: {:?}", ret);
        self.send_report(&ret).await?;
        let ret = self.hid.clear(ReportType::Mouse, report);
        info!("Clear mouse, HID report: {:?}", ret);
        self.send_report(&ret).await?;
        let ret = self.hid.clear(ReportType::Consumer, report);
        info!("Clear consumer, HID report: {:?}", ret);
        self.send_report(&ret).await?;
        Ok(())
    }

    async fn send_status(&mut self, status: IndicatorStatus) -> anyhow::Result<()> {
        let report = &mut [0; 1];
        report[0] = match status {
            IndicatorStatus::WifiConnecting => 0x00,
            IndicatorStatus::WifiConnected => 0x01,
            IndicatorStatus::ServerConnecting => 0x02,
            IndicatorStatus::ServerConnected => 0x03,
            IndicatorStatus::EnterScreen => 0x04,
            IndicatorStatus::LeaveScreen => 0x05,
            IndicatorStatus::ServerDisconnected => 0x06,
            IndicatorStatus::PowerOn => 0x07,
        };
        self.send_report(&(ReportType::Status, report)).await?;
        Ok(())
    }
}

impl Actuator for SerbarActuator {
    async fn connected(&mut self) {
        info!("Connected");
        self.send_status(IndicatorStatus::ServerConnected)
            .await
            .ok();
    }

    async fn disconnected(&mut self) {
        info!("Disconnected");
        self.clear().await.ok();
        self.send_status(IndicatorStatus::ServerDisconnected)
            .await
            .ok();
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
        debug!("Set cursor position to {x} {y}, HID report: {:?}", ret);
        self.send_report(&ret).await.ok();
    }

    async fn move_cursor(&mut self, x: i16, y: i16) {
        self.x = (self.x as i32 + x as i32) as u16;
        self.y = (self.y as i32 + y as i32) as u16;
        self.set_cursor_position(self.x, self.y).await;
    }

    async fn mouse_down(&mut self, button: i8) {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_down(button, report);
        debug!("Mouse button {button} down, HID report: {:?}", ret);
        self.send_report(&ret).await.ok();
    }

    async fn mouse_up(&mut self, button: i8) {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_up(button, report);
        debug!("Mouse button {button} up, HID report: {:?}", ret);
        self.send_report(&ret).await.ok();
    }

    async fn mouse_wheel(&mut self, x: i16, y: i16) {
        let report = &mut [0; 9];
        let ret = self.hid.mouse_scroll(x, y, report);
        debug!("Mouse wheel {x} {y}, HID report: {:?}", ret);
        self.send_report(&ret).await.ok();
    }

    async fn key_down(&mut self, key: u16, mask: u16, button: u16) {
        let report = &mut [0; 9];
        let ret = self.hid.key_down(key, mask, button, report);
        debug!("Key down {key} {mask} {button}, HID report: {:?}", ret);
        self.send_report(&ret).await.ok();
    }

    async fn key_repeat(&mut self, key: u16, mask: u16, button: u16, count: u16) {
        debug!("Key repeat {key} {mask} {button} {count}")
    }

    async fn key_up(&mut self, key: u16, mask: u16, button: u16) {
        let report = &mut [0; 9];
        let ret = self.hid.key_up(key, mask, button, report);
        debug!("Key up {key} {mask} {button}, HID report: {:?}", ret);
        self.send_report(&ret).await.ok();
    }

    async fn enter(&mut self) {
        info!("Enter");
        self.send_status(IndicatorStatus::EnterScreen).await.ok();
    }

    async fn leave(&mut self) {
        info!("Leave");
        self.clear().await.ok();
        self.send_status(IndicatorStatus::LeaveScreen).await.ok();
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
