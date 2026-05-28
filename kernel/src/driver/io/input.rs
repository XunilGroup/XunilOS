use heapless::spsc::{Consumer, Producer, Queue};
use static_cell::StaticCell;

use crate::task::scheduler::SCHEDULER;

pub const KEY_RESERVED: u8 = 0;
pub const KEY_ESC: u8 = 1;
pub const KEY_1: u8 = 2;
pub const KEY_2: u8 = 3;
pub const KEY_3: u8 = 4;
pub const KEY_4: u8 = 5;
pub const KEY_5: u8 = 6;
pub const KEY_6: u8 = 7;
pub const KEY_7: u8 = 8;
pub const KEY_8: u8 = 9;
pub const KEY_9: u8 = 10;
pub const KEY_0: u8 = 11;
pub const KEY_MINUS: u8 = 12;
pub const KEY_EQUAL: u8 = 13;
pub const KEY_BACKSPACE: u8 = 14;
pub const KEY_TAB: u8 = 15;
pub const KEY_Q: u8 = 16;
pub const KEY_W: u8 = 17;
pub const KEY_E: u8 = 18;
pub const KEY_R: u8 = 19;
pub const KEY_T: u8 = 20;
pub const KEY_Y: u8 = 21;
pub const KEY_U: u8 = 22;
pub const KEY_I: u8 = 23;
pub const KEY_O: u8 = 24;
pub const KEY_P: u8 = 25;
pub const KEY_LEFTBRACE: u8 = 26;
pub const KEY_RIGHTBRACE: u8 = 27;
pub const KEY_ENTER: u8 = 28;
pub const KEY_LEFTCTRL: u8 = 29;
pub const KEY_A: u8 = 30;
pub const KEY_S: u8 = 31;
pub const KEY_D: u8 = 32;
pub const KEY_F: u8 = 33;
pub const KEY_G: u8 = 34;
pub const KEY_H: u8 = 35;
pub const KEY_J: u8 = 36;
pub const KEY_K: u8 = 37;
pub const KEY_L: u8 = 38;
pub const KEY_SEMICOLON: u8 = 39;
pub const KEY_APOSTROPHE: u8 = 40;
pub const KEY_GRAVE: u8 = 41;
pub const KEY_LEFTSHIFT: u8 = 42;
pub const KEY_BACKSLASH: u8 = 43;
pub const KEY_Z: u8 = 44;
pub const KEY_X: u8 = 45;
pub const KEY_C: u8 = 46;
pub const KEY_V: u8 = 47;
pub const KEY_B: u8 = 48;
pub const KEY_N: u8 = 49;
pub const KEY_M: u8 = 50;
pub const KEY_COMMA: u8 = 51;
pub const KEY_DOT: u8 = 52;
pub const KEY_SLASH: u8 = 53;
pub const KEY_RIGHTSHIFT: u8 = 54;
pub const KEY_KPASTERISK: u8 = 55;
pub const KEY_LEFTALT: u8 = 56;
pub const KEY_SPACE: u8 = 57;
pub const KEY_CAPSLOCK: u8 = 58;
pub const KEY_F1: u8 = 59;
pub const KEY_F2: u8 = 60;
pub const KEY_F3: u8 = 61;
pub const KEY_F4: u8 = 62;
pub const KEY_F5: u8 = 63;
pub const KEY_F6: u8 = 64;
pub const KEY_F7: u8 = 65;
pub const KEY_F8: u8 = 66;
pub const KEY_F9: u8 = 67;
pub const KEY_F10: u8 = 68;
pub const KEY_NUMLOCK: u8 = 69;
pub const KEY_SCROLLLOCK: u8 = 70;
pub const KEY_KP7: u8 = 71;
pub const KEY_KP8: u8 = 72;
pub const KEY_KP9: u8 = 73;
pub const KEY_KPMINUS: u8 = 74;
pub const KEY_KP4: u8 = 75;
pub const KEY_KP5: u8 = 76;
pub const KEY_KP6: u8 = 77;
pub const KEY_KPPLUS: u8 = 78;
pub const KEY_KP1: u8 = 79;
pub const KEY_KP2: u8 = 80;
pub const KEY_KP3: u8 = 81;
pub const KEY_KP0: u8 = 82;
pub const KEY_KPDOT: u8 = 83;
pub const KEY_RIGHTCTRL: u8 = 97;
pub const KEY_UP: u8 = 103;
pub const KEY_LEFT: u8 = 105;
pub const KEY_RIGHT: u8 = 106;
pub const KEY_DOWN: u8 = 108;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InputEvent {
    pub event_type: u16,
    pub code: u16,
    pub value: u32,
}

pub const EVENT_KEY: u16 = 0x01;
pub const EVENT_REL: u16 = 0x02;

pub const BTN_LEFT: u16 = 0x110;
pub const BTN_RIGHT: u16 = 0x111;
pub const BTN_MIDDLE: u16 = 0x112;

pub const REL_X: u16 = 0x00;
pub const REL_Y: u16 = 0x01;
pub const REL_WHEEL: u16 = 0x08;

pub static INPUT_QUEUE: StaticCell<Queue<InputEvent, 256>> = StaticCell::new();
pub static mut INPUT_PROD: Option<Producer<'static, InputEvent>> = None;
pub static mut INPUT_CONS: Option<Consumer<'static, InputEvent>> = None;

pub fn init_input() {
    let q = INPUT_QUEUE.init(Queue::new());
    let (p, c) = q.split();
    unsafe {
        INPUT_PROD = Some(p);
        INPUT_CONS = Some(c);
    }
}

pub fn process_input() {
    loop {
        let input_event = unsafe {
            #[allow(static_mut_refs)]
            match INPUT_CONS.as_mut() {
                Some(cons) => match cons.dequeue() {
                    Some(b) => b,
                    None => break,
                },
                None => break,
            }
        };

        let mut scheduler = SCHEDULER.lock();
        for process in scheduler.processes.values_mut() {
            process.input_buffer.push(input_event);
        }
        drop(scheduler);
    }
}

pub fn enqueue_input_event(input_event: InputEvent) {
    #[allow(static_mut_refs)]
    let _ = match unsafe { INPUT_PROD.as_mut() } {
        Some(prod) => prod.enqueue(input_event).is_ok(),
        _ => false,
    };
}
