use alloc::vec::Vec;

use crate::{driver::keyboard::KeyboardEvent, mm::address_space::AddressSpace};

pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

pub struct Process {
    pub pid: u64,
    pub state: ProcessState,
    pub stack_top: u64,
    pub heap_base: u64,
    pub heap_end: u64,
    pub kbd_buffer: Vec<KeyboardEvent>,
    pub address_space: AddressSpace,
    pub user_entry: u64,
}
impl Process {
    pub fn new(
        pid: u64,
        user_entry: u64,
        stack_top: u64,
        heap_base: u64,
        heap_end: u64,
    ) -> Option<Process> {
        let address_space = AddressSpace::new()?;

        Some(Process {
            pid,
            stack_top,
            state: ProcessState::Ready,
            heap_base,
            heap_end,
            kbd_buffer: Vec::new(),
            address_space,
            user_entry,
        })
    }
}
