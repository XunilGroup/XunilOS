use core::sync::atomic::{AtomicU64, Ordering};
use heapless::spsc::{Consumer, Producer, Queue};
use pc_keyboard::{DecodedKey, HandleControl, KeyState, Keyboard, ScancodeSet2, layouts};
use static_cell::StaticCell;

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

        return Some(KeyboardEvent {
            state: if state == KeyState::Down { 1 } else { 0 },
            _pad1: 0,
            key: keycode as u16,
            mods: 0,
            _pad2: 0,
            unicode,
        });
    } else {
        return None;
    }
}
