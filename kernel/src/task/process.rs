use core::sync::atomic::Ordering;

use alloc::vec::Vec;

use crate::{
    arch::arch::GLOBAL_TICK_COUNT, driver::io::input::InputEvent, mm::address_space::AddressSpace,
    task::context::UserContext,
};

pub enum ProcessState {
    Ready,
    Running,
    Blocked,
    Zombie,
}

pub struct ProcessInfo {
    pub exit_code: isize,
    pub parent: usize,
    pub wake_tick: Option<u64>,
}

pub struct Process {
    pub pid: u64,
    pub state: ProcessState,
    pub stack_top: u64,
    pub kernel_stack_top: u64,
    pub heap_base: u64,
    pub heap_end: u64,
    pub input_buffer: Vec<InputEvent>,
    pub address_space: Option<AddressSpace>,
    pub saved_ctx: Option<UserContext>,
    pub user_entry: u64,
    pub last_switch_tick: u64,
    pub info: ProcessInfo,
    pub in_ready_queue: bool,
}
impl Process {
    pub fn new(
        pid: u64,
        user_entry: u64,
        stack_top: u64,
        kernel_stack_top: u64,
        heap_base: u64,
        heap_end: u64,
    ) -> Process {
        Process {
            pid,
            stack_top,
            kernel_stack_top,
            state: ProcessState::Ready,
            heap_base,
            heap_end,
            last_switch_tick: GLOBAL_TICK_COUNT.load(Ordering::Relaxed),
            input_buffer: Vec::new(),
            address_space: None,
            saved_ctx: None,
            user_entry,
            info: ProcessInfo {
                exit_code: 0,
                parent: 0,
                wake_tick: None,
            },
            in_ready_queue: false,
        }
    }
}
