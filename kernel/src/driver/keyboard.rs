use alloc::collections::VecDeque;
use lazy_static::lazy_static;
use pc_keyboard::{HandleControl, Keyboard, ScancodeSet1, layouts};
use spin::mutex::Mutex;

#[derive(Debug, Clone)]
pub enum KeyboardEvent {
    Unicode(char),
    RawKey(pc_keyboard::KeyCode),
}

pub struct KeyboardState {
    pub keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    pub event_queue: VecDeque<KeyboardEvent>,
}
impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState {
            keyboard: Keyboard::new(
                ScancodeSet1::new(),
                layouts::Us104Key,
                HandleControl::Ignore,
            ),
            event_queue: VecDeque::new(),
        }
    }
}

lazy_static! {
    pub static ref KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());
}

pub fn pop_event() -> Option<KeyboardEvent> {
    KEYBOARD_STATE.lock().event_queue.pop_front()
}
