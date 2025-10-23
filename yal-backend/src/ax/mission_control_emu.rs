use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::thread;
use std::time::Duration;

const KC_CTRL: CGKeyCode = 59; // kVK_Control
const KC_LEFT: CGKeyCode = 123; // kVK_LeftArrow
const KC_RIGHT: CGKeyCode = 124; // kVK_RightArrow

pub struct MissionControlEmu;

impl MissionControlEmu {
    pub fn new() -> Self {
        Self
    }

    pub fn press_ctrl_digit(&self, n: usize) -> bool {
        let key = match n {
            1 => 18,
            2 => 19,
            3 => 20,
            4 => 21,
            5 => 23,
            6 => 22,
            7 => 26,
            8 => 28,
            9 => 25,
            10 => 29,
            _ => return false,
        };

        let Some(src) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok() else {
            return false;
        };

        if let Ok(e) = CGEvent::new_keyboard_event(src.clone(), KC_CTRL, true) {
            e.post(CGEventTapLocation::HID);
        } else {
            return false;
        }
        thread::sleep(Duration::from_millis(30));

        if let Ok(e) = CGEvent::new_keyboard_event(src.clone(), key, true) {
            e.set_flags(CGEventFlags::CGEventFlagControl);
            e.post(CGEventTapLocation::HID);
        }
        thread::sleep(Duration::from_millis(10));
        if let Ok(e) = CGEvent::new_keyboard_event(src.clone(), key, false) {
            e.set_flags(CGEventFlags::CGEventFlagControl);
            e.post(CGEventTapLocation::HID);
        }
        thread::sleep(Duration::from_millis(10));

        if let Ok(e) = CGEvent::new_keyboard_event(src, KC_CTRL, false) {
            e.post(CGEventTapLocation::HID);
        }
        true
    }

    pub fn press_ctrl_left(&self) -> bool {
        self.ctrl_combo(KC_LEFT)
    }

    pub fn press_ctrl_right(&self) -> bool {
        self.ctrl_combo(KC_RIGHT)
    }

    fn ctrl_combo(&self, key: CGKeyCode) -> bool {
        if !self.post_key(KC_CTRL, true) {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        let _ = self.post_key(key, true);
        std::thread::sleep(std::time::Duration::from_millis(16));
        let _ = self.post_key(key, false);
        std::thread::sleep(std::time::Duration::from_millis(2));
        self.post_key(KC_CTRL, false)
    }

    fn post_key(&self, k: CGKeyCode, down: bool) -> bool {
        let Ok(src) = CGEventSource::new(CGEventSourceStateID::HIDSystemState) else {
            return false;
        };
        if let Ok(e) = CGEvent::new_keyboard_event(src, k, down) {
            e.post(CGEventTapLocation::HID);
            return true;
        }
        false
    }
}
