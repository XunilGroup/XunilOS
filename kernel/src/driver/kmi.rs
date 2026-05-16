use crate::util::get_bit;
use core::sync::atomic::{AtomicI16, AtomicU8, Ordering};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::kmi::{
    read_keyboard_control, read_keyboard_data, read_mouse_control, read_mouse_data,
};
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::kmi::{
    read_keyboard_control, read_keyboard_data, read_mouse_control, read_mouse_data,
};

static CURRENTLY_RECEIVING_STATE: AtomicU8 = AtomicU8::new(0);
static FLAGS_BYTE: AtomicU8 = AtomicU8::new(0);
static X_DELTA_BYTE: AtomicU8 = AtomicU8::new(0);
static Y_DELTA_BYTE: AtomicU8 = AtomicU8::new(0);

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

fn reset_state() {
    CURRENTLY_RECEIVING_STATE.store(0, Ordering::Relaxed);
    FLAGS_BYTE.store(0, Ordering::Relaxed);
    X_DELTA_BYTE.store(0, Ordering::Relaxed);
    Y_DELTA_BYTE.store(0, Ordering::Relaxed);
}

pub fn process_mouse_interrupt() -> Option<(u8, u8, u8, i16, i16)> {
    #[cfg(target_arch = "x86_64")]
    if (unsafe { read_mouse_control() } & 0x20) == 0 {
        return None;
    }
    #[cfg(target_arch = "aarch64")]
    if (unsafe { read_mouse_control() } & 0x10) == 0 {
        return None;
    }

    let byte = unsafe { read_mouse_data() };

    let state_idx = CURRENTLY_RECEIVING_STATE.fetch_add(1, Ordering::Relaxed);

    if state_idx == 0 {
        if (byte & 0x08) == 0 {
            // if sync bit unset, return
            reset_state();
            return None;
        }

        if (byte & 0b0100_0000) != 0 {
            // if x overflow set, return
            reset_state();
            return None;
        }

        if (byte & 0b1000_0000) != 0 {
            // if y overflow set, return
            reset_state();
            return None;
        }

        FLAGS_BYTE.store(byte, Ordering::Relaxed);
        None
    } else if state_idx == 1 {
        X_DELTA_BYTE.store(byte, Ordering::Relaxed);
        None
    } else if state_idx == 2 {
        Y_DELTA_BYTE.store(byte, Ordering::Relaxed);
        let flags = FLAGS_BYTE.load(Ordering::Relaxed);
        let left_button_pressed = get_bit(flags, 0);
        let right_button_pressed = get_bit(flags, 1);
        let middle_button_pressed = get_bit(flags, 2);
        let x_delta_sign = get_bit(flags, 4);
        let y_delta_sign = get_bit(flags, 5);

        let x_delta: i16 = {
            let x_delta = X_DELTA_BYTE.load(Ordering::Relaxed);
            if x_delta_sign == 1 {
                (x_delta as i16) - 256
            } else {
                x_delta as i16
            }
        };

        let y_delta: i16 = -{
            let y_delta = Y_DELTA_BYTE.load(Ordering::Relaxed);
            if y_delta_sign == 1 {
                (y_delta as i16) - 256
            } else {
                y_delta as i16
            }
        };

        reset_state();

        Some((
            left_button_pressed,
            right_button_pressed,
            middle_button_pressed,
            x_delta,
            y_delta,
        ))
    } else {
        None
    }
}

pub fn mouse_interrupt() {
    if let Some(interrupt_result) = process_mouse_interrupt() {
        MOUSE.interrupt(
            interrupt_result.0,
            interrupt_result.1,
            interrupt_result.2,
            interrupt_result.3,
            interrupt_result.4,
        );
    }
}

pub fn keyboard_interrupt() -> Option<u8> {
    #[cfg(target_arch = "x86_64")]
    if (unsafe { read_keyboard_control() } & 0x01) == 0 {
        return None; // OBF clear, no data
    }
    #[cfg(target_arch = "aarch64")]
    if (unsafe { read_keyboard_control() } & 0x10) == 0 {
        return None; // RXFULL clear, no data
    }

    let scancode = unsafe { read_keyboard_data() };
    Some(scancode)
}

pub static MOUSE: Mouse = Mouse::new();
