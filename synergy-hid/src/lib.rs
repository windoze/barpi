use log::{debug, warn};

mod hid;
mod keycodes;

pub(crate) use hid::*;
pub(crate) use keycodes::{synergy_mouse_button, synergy_to_hid, KeyCode};

pub use hid::HID_REPORT_DESCRIPTOR;

#[derive(Debug)]
pub struct SynergyHid {
    width: u16,
    height: u16,
    flip_mouse_wheel: bool,
    x: u16,
    y: u16,
    server_buttons: [u16; 512],

    keyboard_report: KeyboardReport,
    mouse_report: AbsMouseReport,
    consumer_report: ConsumerReport,
}

impl SynergyHid {
    pub fn new(width: u16, height: u16, flip_mouse_wheel: bool) -> Self {
        Self {
            width,
            height,
            flip_mouse_wheel,
            x: 0,
            y: 0,
            server_buttons: [0; 512],
            keyboard_report: KeyboardReport::default(),
            mouse_report: AbsMouseReport::default(),
            consumer_report: ConsumerReport::default(),
        }
    }

    pub fn key_down(&mut self, key: u16, mask: u16, button: u16, report: &mut [u8]) -> usize {
        debug!("Key down {key} {mask} {button}");
        self.server_buttons[button as usize] = key;
        let hid = synergy_to_hid(key);
        debug!("Key Down {:#04x} -> Keycode: {:?}", key, hid);
        match hid {
            KeyCode::None => {
                warn!("Keycode not found");
                report[0] = 0x01;
                report[1..9].copy_from_slice(&self.keyboard_report.clear());
                9
            }
            KeyCode::Key(key) => {
                report[0] = 0x01;
                report[1..9].copy_from_slice(&self.keyboard_report.press(key));
                9
            }
            KeyCode::Consumer(key) => {
                report[0] = 0x03;
                report[1..3].copy_from_slice(&self.consumer_report.press(key));
                3
            }
        }
    }

    pub fn key_up(&mut self, key: u16, mask: u16, button: u16, report: &mut [u8]) -> usize {
        debug!("Key down {key} {mask} {button}");
        let key = self.server_buttons[button as usize];
        let hid = if self.server_buttons[button as usize] != 0 {
            debug!("Key {key} up");
            self.server_buttons[button as usize] = 0;
            synergy_to_hid(key)
        } else if key == 0 {
            debug!("Key 0 up, clear all key down");
            KeyCode::None
        } else {
            warn!("Key {key} up with no key down");
            KeyCode::None
        };
        debug!("Key Down {:#04x} -> Keycode: {:?}", key, hid);
        match hid {
            KeyCode::None => {
                warn!("Keycode not found");
                report[0] = 0x01;
                report[1..9].copy_from_slice(&self.keyboard_report.clear());
                9
            }
            KeyCode::Key(key) => {
                report[0] = 0x01;
                report[1..9].copy_from_slice(&self.keyboard_report.release(key));
                9
            }
            KeyCode::Consumer(_key) => {
                report[0] = 0x03;
                report[1..3].copy_from_slice(&self.consumer_report.release());
                3
            }
        }
    }

    pub fn set_cursor_position(&mut self, x: u16, y: u16, report: &mut [u8]) -> usize {
        (self.x, self.y) = self.scale_position(x, y);
        let (x, y) = self.scale_position(x, y);
        report[0] = 0x02;
        report[1..8].copy_from_slice(&self.mouse_report.move_to(x, y));
        8
    }

    pub fn move_cursor(&mut self, x: i16, y: i16, report: &mut [u8]) -> usize {
        self.set_cursor_position(
            (self.x as i32 + x as i32) as u16,
            (self.y as i32 + y as i32) as u16,
            report,
        )
    }

    pub fn mouse_down(&mut self, button: i8, report: &mut [u8]) -> usize {
        report[0] = 0x02;
        report[1..8].copy_from_slice(&self.mouse_report.mouse_down(synergy_mouse_button(button)));
        8
    }

    pub fn mouse_up(&mut self, button: i8, report: &mut [u8]) -> usize {
        report[0] = 0x02;
        report[1..8].copy_from_slice(&self.mouse_report.mouse_up(synergy_mouse_button(button)));
        8
    }

    pub fn mouse_scroll(&mut self, x: i16, y: i16, report: &mut [u8]) -> usize {
        let x = x as i8;
        let mut y = y as i8;
        if self.flip_mouse_wheel {
            y = -y;
        }
        report[0] = 0x02;
        report[1..8].copy_from_slice(&self.mouse_report.mouse_wheel(y, x));
        8
    }

    fn scale_position(&self, x: u16, y: u16) -> (u16, u16) {
        // NOTE: Some errors could be introduced without `ceil`, but shouldn't be a big deal.
        (
            ((x as f32) * (0x7fff as f32 / (self.width as f32))).ceil() as u16,
            ((y as f32) * (0x7fff as f32 / (self.height as f32))).ceil() as u16,
        )
    }
}

#[cfg(test)]
mod test {
    use crate::keycodes::{HID_KEY_A, HID_KEY_B};

    #[test]
    fn test_key() {
        let mut hid = super::SynergyHid::new(1920, 1080, false);
        let mut report = [0; 9];
        let sz = hid.key_down(0x0000, 0x0000, 0x0000, &mut report);
        assert_eq!(sz, 9);
        assert_eq!(report, [0x01, 0, 0, 0, 0, 0, 0, 0, 0]);
        hid.key_down('A' as u16, 0x0000, 0x0000, &mut report);
        assert_eq!(report, [0x01, 0, 0, HID_KEY_A, 0, 0, 0, 0, 0]);
        hid.key_down('B' as u16, 0x0000, 0x0000, &mut report);
        assert_eq!(report, [0x01, 0, 0, HID_KEY_A, HID_KEY_B, 0, 0, 0, 0]);
        hid.key_up('B' as u16, 0x0000, 0x0000, &mut report);
        assert_eq!(report, [0x01, 0, 0, HID_KEY_A, 0, 0, 0, 0, 0]);
        // Wrong key up, report is cleared
        hid.key_up('C' as u16, 0x0000, 0x0000, &mut report);
        assert_eq!(report, [0x01, 0, 0, 0, 0, 0, 0, 0, 0]);

        // kKeyAudioMute(0xE0AD) -> HID_USAGE_CONSUMER_MUTE(0x00E2)
        let sz = hid.key_down(0xE0AD, 0x0000, 1, &mut report);
        assert_eq!(sz, 3);
        assert_eq!(report[0..sz], [0x03, 0x00, 0xE2]);
    }
}
