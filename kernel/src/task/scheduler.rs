use core::sync::atomic::{AtomicU64, Ordering};

use alloc::collections::btree_map::BTreeMap;
use x86_64::instructions::interrupts::without_interrupts;

use crate::{arch::arch::enter_usermode, task::process::Process, util::Locked};

pub static CURRENT_PID: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn current_pid() -> Option<u64> {
    match CURRENT_PID.load(Ordering::Relaxed) {
        0 => None,
        pid => Some(pid),
    }
}

#[inline]
pub fn set_current_pid(pid: Option<u64>) {
    CURRENT_PID.store(pid.unwrap_or(0), Ordering::Relaxed);
}

pub struct Scheduler {
    pub processes: BTreeMap<u64, Process>,
    next_pid: u64,
}

impl Scheduler {
    pub const fn new() -> Scheduler {
        Scheduler {
            processes: BTreeMap::new(),
            next_pid: 1,
        }
    }
}

impl Locked<Scheduler> {
    pub fn spawn_process(&self, entry_point: u64, stack_top: u64, heap_base: u64) -> Option<u64> {
        let mut guard = without_interrupts(|| self.lock());
        let pid = guard.next_pid;
        guard.next_pid += 1;
        let process = Process::new(pid, entry_point, stack_top, heap_base, heap_base);
        guard.processes.insert(pid, process);

        Some(pid)
    }

    pub fn run_process(&self, pid: u64, entry_point: *const u8) {
        let stack_top = {
            let guard = without_interrupts(|| self.lock());
            guard.processes[&pid].stack_top
        };

        set_current_pid(Some(pid));
        enter_usermode(entry_point as u64, (stack_top & !0xF) - 8);
    }

    pub fn with_process<F, R>(&self, index: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut Process) -> R,
    {
        let mut guard = without_interrupts(|| self.lock());
        let process = guard.processes.get_mut(&index)?;
        Some(f(process))
    }
}

pub static SCHEDULER: Locked<Scheduler> = Locked::new(Scheduler::new());
