use heapless::spsc::{Consumer, Producer, Queue};
use static_cell::StaticCell;

use crate::{
    arch::arch::serial_print,
    driver::io::{
        keyboard::{
            KEY_CAPSLOCK, KEY_LEFTALT, KEY_LEFTCTRL, KEY_LEFTSHIFT, KEY_RIGHTSHIFT, KeyboardEvent,
            keycode_to_char,
        },
        mouse::MOUSE,
        virtio::{
            KEYBOARD_SLOT, MOUSE_SLOT, VIRTIO_MMIO_BASE, VIRTIO_MMIO_STRIDE,
            queue::{QUEUE_SIZE, VirtqueueMem},
            transport::{VirtioMmio, VirtioMmioReg},
        },
    },
    task::scheduler::SCHEDULER,
};

const BTN_LEFT: u16 = 0x110;
const BTN_RIGHT: u16 = 0x111;
const BTN_MIDDLE: u16 = 0x112;

const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const REL_WHEEL: u16 = 0x08;

const EVENT_KEY: u16 = 0x01;
const EVENT_REL: u16 = 0x02;

#[repr(u8)]
pub enum VirtioInputCfgSelect {
    Unset = 0x00,
    IdName = 0x01,
    IdSerial = 0x02,
    IdDevids = 0x03,
    PropBits = 0x10,
    EvBits = 0x11,
    AbsInfo = 0x12,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtioInputEvent {
    event_type: u16,
    code: u16,
    value: u32,
}

pub struct ModState {
    shift: bool,
    caps_lock: bool,
    ctrl: bool,
    alt: bool,
}

impl ModState {
    pub const fn new() -> Self {
        Self {
            shift: false,
            caps_lock: false,
            ctrl: false,
            alt: false,
        }
    }
    pub fn update(&mut self, code: u8, value: u32) {
        match code {
            KEY_LEFTSHIFT | KEY_RIGHTSHIFT => self.shift = value != 0,
            KEY_LEFTCTRL => self.ctrl = value != 0,
            KEY_CAPSLOCK => self.caps_lock = value != 0,
            KEY_LEFTALT => self.alt = value != 0,
            _ => {}
        }
    }

    pub fn effective_shift(&self) -> bool {
        self.shift ^ self.caps_lock
    }
}

pub static mut VIRTIO_KEYBOARD_QUEUE: VirtqueueMem = unsafe { core::mem::zeroed() };
pub static mut VIRTIO_MOUSE_QUEUE: VirtqueueMem = unsafe { core::mem::zeroed() };
pub static mut MODIFIERS: ModState = ModState::new();

static KEYCODE_QUEUE: StaticCell<Queue<(u8, u8, bool), 256>> = StaticCell::new();
static mut KEYCODE_PROD: Option<Producer<'static, (u8, u8, bool)>> = None;
static mut KEYCODE_CONS: Option<Consumer<'static, (u8, u8, bool)>> = None;

pub fn init_keyboard() {
    let q = KEYCODE_QUEUE.init(Queue::new());
    let (p, c) = q.split();

    unsafe {
        KEYCODE_PROD = Some(p);
        KEYCODE_CONS = Some(c);
    }
}

pub fn process_keycodes() {
    let mut scheduler = SCHEDULER.lock();

    loop {
        let keycode_data = unsafe {
            #[allow(static_mut_refs)]
            match KEYCODE_CONS.as_mut() {
                Some(cons) => match cons.dequeue() {
                    Some(b) => b,
                    None => break,
                },
                None => break,
            }
        };

        for process in scheduler.processes.values_mut() {
            process.kbd_buffer.push(KeyboardEvent {
                state: keycode_data.1,
                _pad1: 0,
                key: keycode_data.0 as u16,
                mods: 0,
                _pad2: 0,
                unicode: keycode_to_char(keycode_data.0, keycode_data.2).unwrap_or('\0') as u32,
            });
        }
    }

    drop(scheduler);
}

pub fn read_device_name(device: &VirtioMmio) -> &str {
    return unsafe {
        let select_ptr = (device.base + 0x100) as *mut u8;
        let subsel_ptr = (device.base + 0x101) as *mut u8;
        let size_ptr = (device.base + 0x102) as *mut u8;
        let name_ptr = (device.base + 0x108) as *mut u8;

        select_ptr.write_volatile(VirtioInputCfgSelect::IdName as u8);
        subsel_ptr.write_volatile(0x00);

        let size = size_ptr.read_volatile() as usize;

        if size == 0 || size > 128 {
            return "Error";
        }

        let slice = core::slice::from_raw_parts(name_ptr, size);
        core::str::from_utf8(slice).unwrap_or("Error")
    };
}

pub fn input_interrupt(device_type: &str) {
    #[allow(static_mut_refs)]
    let queue: Option<&mut VirtqueueMem> = {
        if device_type == "kbd" {
            Some(unsafe { &mut VIRTIO_KEYBOARD_QUEUE })
        } else if device_type == "mouse" {
            Some(unsafe { &mut VIRTIO_MOUSE_QUEUE })
        } else {
            None
        }
    };

    let device = VirtioMmio::new(
        VIRTIO_MMIO_BASE + {
            if device_type == "kbd" {
                KEYBOARD_SLOT.load(core::sync::atomic::Ordering::Relaxed)
            } else if device_type == "mouse" {
                MOUSE_SLOT.load(core::sync::atomic::Ordering::Relaxed)
            } else {
                0
            }
        } * VIRTIO_MMIO_STRIDE,
    );

    if queue.is_none() {
        return;
    }

    let queue = queue.unwrap();

    let status = device.read(VirtioMmioReg::InterruptStatus);
    device.write(VirtioMmioReg::InterruptAck, status);

    loop {
        let used_idx = unsafe { core::ptr::read_volatile(&queue.used.idx) };
        let last_idx = queue.last_used_idx;

        if last_idx == used_idx {
            return;
        }

        let used_element = &queue.used.ring[last_idx as usize % QUEUE_SIZE];
        let desc_idx = used_element.id;

        queue.last_used_idx = last_idx.wrapping_add(1);

        let event = unsafe { core::ptr::read_volatile(&queue.buffers[desc_idx as usize]) };

        handle_event(&event);

        let avail_idx = unsafe { core::ptr::read_volatile(&queue.avail.idx) };
        queue.avail.ring[(avail_idx as usize) % QUEUE_SIZE] = desc_idx as u16;

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        unsafe {
            core::ptr::write_volatile(&mut queue.avail.idx, avail_idx.wrapping_add(1));
        }
    }
}

#[allow(static_mut_refs)]
pub fn handle_event(event: &VirtioInputEvent) {
    if event.event_type == 0 || event.code == 0 {
        return;
    }

    match event.event_type {
        EVENT_KEY => {
            unsafe { MODIFIERS.update(event.code as u8, event.value) };
            match event.code {
                BTN_LEFT => {
                    MOUSE
                        .left_button_pressed
                        .store(event.value == 1, core::sync::atomic::Ordering::Relaxed);
                }
                BTN_RIGHT => {
                    MOUSE
                        .right_button_pressed
                        .store(event.value == 1, core::sync::atomic::Ordering::Relaxed);
                }
                BTN_MIDDLE => {
                    MOUSE
                        .middle_button_pressed
                        .store(event.value == 1, core::sync::atomic::Ordering::Relaxed);
                }
                _ => {
                    let state = match event.value {
                        1 => 1,
                        0 => 0,
                        _ => return,
                    };

                    let _ = match unsafe { KEYCODE_PROD.as_mut() } {
                        Some(prod) => prod
                            .enqueue(unsafe {
                                (event.code as u8, state, MODIFIERS.effective_shift())
                            })
                            .is_ok(),
                        _ => false,
                    };
                }
            }
        }
        EVENT_REL => match event.code {
            REL_X => {
                MOUSE.x_delta.fetch_add(
                    event.value as i32 as i16,
                    core::sync::atomic::Ordering::Relaxed,
                );
            }
            REL_Y => {
                MOUSE.y_delta.fetch_add(
                    event.value as i32 as i16,
                    core::sync::atomic::Ordering::Relaxed,
                );
            }
            REL_WHEEL => {}
            _ => {}
        },
        _ => serial_print("Could not recognize virtio input event from interrupt\n"),
    };
}
