use core::sync::atomic::{AtomicU64, Ordering};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    arch::arch::{enter_usermode, safe_lock},
    task::context::UserContext,
    task::process::{Process, ProcessState},
    util::Locked,
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::interrupts::run_next;
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::syscall::run_next;

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
        let mut guard = safe_lock(|| self.lock());
        let pid = guard.next_pid;
        guard.next_pid += 1;
        let process = Process::new(pid, entry_point, stack_top, heap_base, heap_base);
        guard.processes.insert(pid, process);

        Some(pid)
    }

    pub fn next_task(&self) -> u64 {
        if let Some(previous_pid) = current_pid() {
            let mut guard = safe_lock(|| self.lock());

            if let Some(process) = guard.processes.get_mut(&previous_pid) {
                if matches!(process.state, ProcessState::Running) {
                    process.state = ProcessState::Ready;
                }
            }

            let ready_pids: Vec<u64> = guard
                .processes
                .iter()
                .filter(|(_, process)| matches!(process.state, ProcessState::Ready))
                .map(|(&pid, _)| pid)
                .collect();

            if ready_pids.is_empty() {
                return previous_pid;
            }

            let current_index = ready_pids.iter().position(|&pid| pid == previous_pid);

            return match current_index {
                Some(i) => {
                    let next_index = (i + 1) % ready_pids.len();
                    ready_pids[next_index]
                }
                None => ready_pids[0],
            };
        } else {
            panic!("Could not get current PID when switching to next task")
        };
    }

    pub fn switch_to(&self, pid: u64, should_swapgs: bool) {
        let (ctx_opt, entry, stack_top) = {
            let mut guard = safe_lock(|| self.lock());

            if let Some(previous_pid) = current_pid() {
                if let Some(old_process) = guard.processes.get_mut(&previous_pid) {
                    if matches!(old_process.state, ProcessState::Running) {
                        old_process.state = ProcessState::Ready;
                    }
                } else {
                    // no previous process
                }
            }

            let new_process = guard.processes.get_mut(&pid).expect("Cant get new process");
            new_process.state = ProcessState::Running;
            if let Some(address_space) = new_process.address_space.as_mut() {
                address_space.use_address_space();
            };

            (
                new_process.saved_ctx,
                new_process.user_entry,
                new_process.stack_top,
            )
        };

        set_current_pid(Some(pid));

        match ctx_opt {
            #[allow(unused_variables, unused_unsafe)]
            Some(saved_ctx) => unsafe {
                #[cfg(target_arch = "x86_64")]
                run_next((&saved_ctx) as *const UserContext, saved_ctx.rsp);
                #[cfg(target_arch = "aarch64")]
                run_next((&saved_ctx) as *const UserContext, saved_ctx.sp_el0);
            },
            None => enter_usermode(
                entry as u64,
                (stack_top & !0xF)
                    - cfg_select! {
                        target_arch = "x86_64" => 8,
                        target_arch = "aarch64" => 16,
                        _ => 8
                    },
                should_swapgs,
            ),
        }
    }

    pub fn with_process<F, R>(&self, index: u64, f: F) -> Option<R>
    where
        F: FnOnce(&mut Process) -> R,
    {
        let mut guard = safe_lock(|| self.lock());
        let process = guard.processes.get_mut(&index)?;
        Some(f(process))
    }
}

pub static SCHEDULER: Locked<Scheduler> = Locked::new(Scheduler::new());
