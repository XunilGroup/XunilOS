use alloc::collections::btree_map::BTreeMap;
use lazy_static::lazy_static;

use crate::{arch::x86_64::paging::XunilFrameAllocator, task::process::Process, util::Locked};

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
    pub fn spawn_process(
        &self,
        entry_point: u64,
        frame_allocator: &mut XunilFrameAllocator,
    ) -> Option<u64> {
        let mut guard = self.lock();
        let pid = guard.next_pid;
        guard.next_pid += 1;
        let process = Process::new(pid, entry_point, frame_allocator)?;
        guard.processes.insert(pid, process);

        Some(pid)
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
