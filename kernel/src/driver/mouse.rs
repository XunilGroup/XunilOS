use core::sync::atomic::{AtomicI16, AtomicU8, Ordering};

pub struct Mouse {
    left_button_pressed: AtomicU8,
    right_button_pressed: AtomicU8,
    middle_button_pressed: AtomicU8,
    x_delta: AtomicI16,
    y_delta: AtomicI16,
    status: AtomicU8,
}

impl Mouse {
    const fn new() -> Mouse {
        Mouse {
            left_button_pressed: AtomicU8::new(0),
            right_button_pressed: AtomicU8::new(0),
            middle_button_pressed: AtomicU8::new(0),
            x_delta: AtomicI16::new(0),
            y_delta: AtomicI16::new(0),
            status: AtomicU8::new(0),
        }
    }

    pub fn interrupt(
        &self,
        left_button_pressed: u8,
        right_button_pressed: u8,
        middle_button_pressed: u8,
        x_delta: i16,
        y_delta: i16,
    ) {
        self.left_button_pressed
            .store(left_button_pressed, Ordering::Relaxed);
        self.right_button_pressed
            .store(right_button_pressed, Ordering::Relaxed);
        self.middle_button_pressed
            .store(middle_button_pressed, Ordering::Relaxed);
        self.x_delta.fetch_add(x_delta, Ordering::Relaxed);
        self.y_delta.fetch_add(y_delta, Ordering::Relaxed);
    }

    pub fn button_state(&self) -> (u8, u8, u8) {
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
