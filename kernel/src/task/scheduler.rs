use alloc::collections::btree_map::BTreeMap;
use lazy_static::lazy_static;

use crate::{
    arch::{arch::enter_usermode, x86_64::paging::XunilFrameAllocator},
    task::process::Process,
    util::Locked,
};

pub struct Scheduler {
    pub processes: BTreeMap<u64, Process>,
    pub current_process: i64,
    next_pid: u64,
}

impl Scheduler {
    pub const fn new() -> Scheduler {
        Scheduler {
            processes: BTreeMap::new(),
            current_process: -1,
            next_pid: 1,
        }
    }
}

impl Locked<Scheduler> {
    pub fn spawn_process(&self, entry_point: u64, stack_top: u64, heap_base: u64) -> Option<u64> {
        let mut guard = self.lock();
        let pid = guard.next_pid;
        guard.next_pid += 1;
        let process = Process::new(pid, entry_point, stack_top, heap_base, heap_base)?;
        guard.processes.insert(pid, process);

        Some(pid)
    }

    pub fn run_process(&self, pid: u64, entry_point: *const u8) {
        let mut guard = self.lock();
        let stack_top = guard.processes[&pid].stack_top;
        guard.current_process = pid as i64;

        enter_usermode(entry_point as u64, (stack_top & !0xF) - 8);
    }

    pub fn with_process<F, R>(&self, index: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut Process) -> R,
    {
        let mut guard = self.lock();
        let process = guard.processes.get_mut(&index)?;
        Some(f(process))
    }
}

pub static SCHEDULER: Locked<Scheduler> = Locked::new(Scheduler::new());
