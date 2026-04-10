use pc_keyboard::{DecodedKey, HandleControl, KeyState, Keyboard, ScancodeSet2, layouts};
use spin::mutex::Mutex;
use x86_64::instructions::interrupts::without_interrupts;

use crate::task::scheduler::SCHEDULER;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct KeyboardEvent {
    pub state: u8,
    pub _pad1: u8,
    pub key: u16,
    pub mods: u16,
    pub _pad2: u16,
    pub unicode: u32,
}

pub fn process_keyboard_event(scancode: u8) {
    let mut keyboard = without_interrupts(|| KEYBOARD.lock());

    let scheduler = without_interrupts(|| SCHEDULER.lock());
    let pid = scheduler.current_process;

    if pid < 0 {
        return;
    }

    drop(scheduler);

    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        let keycode = key_event.code;
        let keystate = key_event.state;
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    SCHEDULER.with_process(pid as u64, |process| {
                        process.kbd_buffer.push(KeyboardEvent {
                            state: if keystate == KeyState::Down { 1 } else { 0 },
                            _pad1: 0,
                            key: keycode as u16,
                            mods: 0,
                            _pad2: 0,
                            unicode: character as u32,
                        })
                    });
                }
                DecodedKey::RawKey(_) => {
                    SCHEDULER.with_process(pid as u64, |process| {
                        process.kbd_buffer.push(KeyboardEvent {
                            state: if keystate == KeyState::Down { 1 } else { 0 },
                            _pad1: 0,
                            key: keycode as u16,
                            mods: 0,
                            _pad2: 0,
                            unicode: 0,
                        })
                    });
                }
            }
        } else {
            SCHEDULER.with_process(pid as u64, |process| {
                process.kbd_buffer.push(KeyboardEvent {
                    state: if keystate == KeyState::Down { 1 } else { 0 },
                    _pad1: 0,
                    key: keycode as u16,
                    mods: 0,
                    _pad2: 0,
                    unicode: 0,
                })
            });
        }
    }
}

pub static KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet2>> = Mutex::new(Keyboard::new(
    ScancodeSet2::new(),
    layouts::Us104Key,
    HandleControl::Ignore,
));
