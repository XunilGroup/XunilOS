use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use heapless::spsc::{Consumer, Producer, Queue};
use pc_keyboard::{DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet2, layouts};

use static_cell::StaticCell;

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::kmi::{
    read_keyboard_control, read_keyboard_data, read_mouse_control, read_mouse_data,
};
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::kmi::{
    read_keyboard_control, read_keyboard_data, read_mouse_control, read_mouse_data,
};
use crate::{
    driver::io::{keyboard::*, mouse::MOUSE},
    println,
    task::scheduler::SCHEDULER,
    util::get_bit,
};

pub fn keycode_to_linux(kc: KeyCode) -> Option<u8> {
    let code = match kc {
        KeyCode::Escape => KEY_ESC,
        KeyCode::Key1 => KEY_1,
        KeyCode::Key2 => KEY_2,
        KeyCode::Key3 => KEY_3,
        KeyCode::Key4 => KEY_4,
        KeyCode::Key5 => KEY_5,
        KeyCode::Key6 => KEY_6,
        KeyCode::Key7 => KEY_7,
        KeyCode::Key8 => KEY_8,
        KeyCode::Key9 => KEY_9,
        KeyCode::Key0 => KEY_0,
        KeyCode::OemMinus => KEY_MINUS,
        KeyCode::OemPlus => KEY_EQUAL,
        KeyCode::Backspace => KEY_BACKSPACE,
        KeyCode::Tab => KEY_TAB,
        KeyCode::Q => KEY_Q,
        KeyCode::W => KEY_W,
        KeyCode::E => KEY_E,
        KeyCode::R => KEY_R,
        KeyCode::T => KEY_T,
        KeyCode::Y => KEY_Y,
        KeyCode::U => KEY_U,
        KeyCode::I => KEY_I,
        KeyCode::O => KEY_O,
        KeyCode::P => KEY_P,
        KeyCode::Oem4 => KEY_LEFTBRACE,
        KeyCode::Oem6 => KEY_RIGHTBRACE,
        KeyCode::Return => KEY_ENTER,
        KeyCode::LControl => KEY_LEFTCTRL,
        KeyCode::A => KEY_A,
        KeyCode::S => KEY_S,
        KeyCode::D => KEY_D,
        KeyCode::F => KEY_F,
        KeyCode::G => KEY_G,
        KeyCode::H => KEY_H,
        KeyCode::J => KEY_J,
        KeyCode::K => KEY_K,
        KeyCode::L => KEY_L,
        KeyCode::Oem1 => KEY_SEMICOLON,
        KeyCode::Oem7 => KEY_APOSTROPHE,
        KeyCode::LShift => KEY_LEFTSHIFT,
        KeyCode::Oem5 => KEY_BACKSLASH,
        KeyCode::Z => KEY_Z,
        KeyCode::X => KEY_X,
        KeyCode::C => KEY_C,
        KeyCode::V => KEY_V,
        KeyCode::B => KEY_B,
        KeyCode::N => KEY_N,
        KeyCode::M => KEY_M,
        KeyCode::OemComma => KEY_COMMA,
        KeyCode::OemPeriod => KEY_DOT,
        KeyCode::Oem2 => KEY_SLASH,
        KeyCode::RShift => KEY_RIGHTSHIFT,
        KeyCode::NumpadMultiply => KEY_KPASTERISK,
        KeyCode::LAlt => KEY_LEFTALT,
        KeyCode::Spacebar => KEY_SPACE,
        KeyCode::CapsLock => KEY_CAPSLOCK,
        KeyCode::F1 => KEY_F1,
        KeyCode::F2 => KEY_F2,
        KeyCode::F3 => KEY_F3,
        KeyCode::F4 => KEY_F4,
        KeyCode::F5 => KEY_F5,
        KeyCode::F6 => KEY_F6,
        KeyCode::F7 => KEY_F7,
        KeyCode::F8 => KEY_F8,
        KeyCode::F9 => KEY_F9,
        KeyCode::F10 => KEY_F10,
        KeyCode::NumpadLock => KEY_NUMLOCK,
        KeyCode::ScrollLock => KEY_SCROLLLOCK,
        KeyCode::Numpad7 => KEY_KP7,
        KeyCode::Numpad8 => KEY_KP8,
        KeyCode::Numpad9 => KEY_KP9,
        KeyCode::NumpadSubtract => KEY_KPMINUS,
        KeyCode::Numpad4 => KEY_KP4,
        KeyCode::Numpad5 => KEY_KP5,
        KeyCode::Numpad6 => KEY_KP6,
        KeyCode::NumpadAdd => KEY_KPPLUS,
        KeyCode::Numpad1 => KEY_KP1,
        KeyCode::Numpad2 => KEY_KP2,
        KeyCode::Numpad3 => KEY_KP3,
        KeyCode::Numpad0 => KEY_KP0,
        KeyCode::NumpadPeriod => KEY_KPDOT,
        KeyCode::RControl => KEY_RIGHTCTRL,
        KeyCode::ArrowUp => KEY_UP,
        KeyCode::ArrowLeft => KEY_LEFT,
        KeyCode::ArrowRight => KEY_RIGHT,
        KeyCode::ArrowDown => KEY_DOWN,
        _ => return None,
    };
    Some(code)
}

static SCANCODE_QUEUE: StaticCell<Queue<u8, 256>> = StaticCell::new();
static mut SCANCODE_PROD: Option<Producer<'static, u8>> = None;
static mut SCANCODE_CONS: Option<Consumer<'static, u8>> = None;
static mut KEYBOARD: Option<Keyboard<layouts::Us104Key, ScancodeSet2>> = None;

static DROPPED_SCANCODES: AtomicU64 = AtomicU64::new(0);

pub fn init_keyboard() {
    let q = SCANCODE_QUEUE.init(Queue::new());
    let (p, c) = q.split();

    unsafe {
        SCANCODE_PROD = Some(p);
        SCANCODE_CONS = Some(c);
        KEYBOARD = Some(Keyboard::new(
            ScancodeSet2::new(),
            layouts::Us104Key,
            HandleControl::Ignore,
        ))
    }
}

pub fn push_scancode(scancode: u8) {
    let pushed = unsafe {
        #[allow(static_mut_refs)]
        match SCANCODE_PROD.as_mut() {
            Some(prod) => prod.enqueue(scancode).is_ok(),
            _ => false,
        }
    };

    if !pushed {
        DROPPED_SCANCODES.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn process_scancodes() {
    loop {
        let scancode = unsafe {
            #[allow(static_mut_refs)]
            match SCANCODE_CONS.as_mut() {
                Some(cons) => match cons.dequeue() {
                    Some(b) => b,
                    None => break,
                },
                None => break,
            }
        };

        if let Some(kbd_event) = process_scancode(scancode) {
            let mut scheduler = SCHEDULER.lock();
            for process in scheduler.processes.values_mut() {
                process.kbd_buffer.push(kbd_event);
            }
            drop(scheduler);
        }
    }
}

pub fn process_scancode(scancode: u8) -> Option<KeyboardEvent> {
    #[allow(static_mut_refs)]
    let kbd = unsafe { KEYBOARD.as_mut().expect("keyboard not initialized") };
    if let Ok(Some(key_event)) = kbd.add_byte(scancode) {
        let keycode = key_event.code;
        let keystate = key_event.state;
        let (unicode, state) = match (kbd.process_keyevent(key_event), keystate) {
            (Some(DecodedKey::Unicode(ch)), st) => (ch as u32, st),
            _ => (0, keystate),
        };

        if let Some(linux_keycode) = keycode_to_linux(keycode) {
            let effective_shift = kbd.get_modifiers().is_shifted() & kbd.get_modifiers().capslock;
            return Some(KeyboardEvent {
                state: if state == KeyState::Down { 1 } else { 0 },
                _pad1: 0,
                key: linux_keycode as u16,
                mods: 0,
                _pad2: 0,
                unicode: keycode_to_char(linux_keycode, effective_shift).unwrap_or('\0') as u32,
            });
        } else {
            return None;
        }
    } else {
        return None;
    }
}

static CURRENTLY_RECEIVING_STATE: AtomicU8 = AtomicU8::new(0);
static FLAGS_BYTE: AtomicU8 = AtomicU8::new(0);
static X_DELTA_BYTE: AtomicU8 = AtomicU8::new(0);
static Y_DELTA_BYTE: AtomicU8 = AtomicU8::new(0);

fn reset_state() {
    CURRENTLY_RECEIVING_STATE.store(0, Ordering::Relaxed);
    FLAGS_BYTE.store(0, Ordering::Relaxed);
    X_DELTA_BYTE.store(0, Ordering::Relaxed);
    Y_DELTA_BYTE.store(0, Ordering::Relaxed);
}

pub fn process_mouse_interrupt() -> Option<(bool, bool, bool, i16, i16)> {
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
            left_button_pressed == 1,
            right_button_pressed == 1,
            middle_button_pressed == 1,
            x_delta,
            y_delta,
        ))
    } else {
        None
    }
}

pub fn mouse_interrupt() {
    if let Some(interrupt_result) = process_mouse_interrupt() {
        MOUSE
            .left_button_pressed
            .store(interrupt_result.0, Ordering::Relaxed);
        MOUSE
            .right_button_pressed
            .store(interrupt_result.1, Ordering::Relaxed);
        MOUSE
            .middle_button_pressed
            .store(interrupt_result.2, Ordering::Relaxed);
        MOUSE
            .x_delta
            .fetch_add(interrupt_result.3, Ordering::Relaxed);
        MOUSE
            .y_delta
            .fetch_add(interrupt_result.4, Ordering::Relaxed);
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
