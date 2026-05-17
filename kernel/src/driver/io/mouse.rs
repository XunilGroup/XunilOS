use core::sync::atomic::{AtomicBool, AtomicI16, AtomicU8, Ordering};

pub struct Mouse {
    pub left_button_pressed: AtomicBool,
    pub right_button_pressed: AtomicBool,
    pub middle_button_pressed: AtomicBool,
    pub x_delta: AtomicI16,
    pub y_delta: AtomicI16,
    pub status: AtomicU8,
}

impl Mouse {
    const fn new() -> Mouse {
        Mouse {
            left_button_pressed: AtomicBool::new(false),
            right_button_pressed: AtomicBool::new(false),
            middle_button_pressed: AtomicBool::new(false),
            x_delta: AtomicI16::new(0),
            y_delta: AtomicI16::new(0),
            status: AtomicU8::new(0),
        }
    }

    pub fn button_state(&self) -> (bool, bool, bool) {
        (
            self.left_button_pressed.load(Ordering::Relaxed),
            self.right_button_pressed.load(Ordering::Relaxed),
            self.middle_button_pressed.load(Ordering::Relaxed),
        )
    }
    pub fn take_motion(&self) -> (i16, i16) {
        (
            self.x_delta.swap(0, Ordering::Relaxed),
            self.y_delta.swap(0, Ordering::Relaxed),
        )
    }
    pub fn set_status(&self, status: u8) {
        self.status.store(status, Ordering::Relaxed);
    }
    pub fn get_status(&self) -> u8 {
        self.status.load(Ordering::Relaxed)
    }
}

pub static MOUSE: Mouse = Mouse::new();
