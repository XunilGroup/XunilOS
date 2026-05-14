use core::sync::atomic::{AtomicU64, Ordering};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    arch::arch::{enter_usermode, safe_lock},
    task::{
        context::UserContext,
        process::{Process, ProcessState},
    },
    util::Locked,
};

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

    #[cfg(target_arch = "x86_64")]
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
            Some(saved_ctx) => unsafe {
                run_next((&saved_ctx) as *const UserContext, saved_ctx.rsp)
            },
            None => enter_usermode(entry as u64, (stack_top & !0xF) - 8, should_swapgs),
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

#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe fn run_next(ctx: *const UserContext, user_rsp: u64) {
    core::arch::naked_asm!(
        "mov gs:[0], rsi", // store new user rsp
        "mov rsp, rdi",
        "mov r15, qword ptr [rsp + 0]",
        "mov r14, qword ptr [rsp + 8]",
        "mov r13, qword ptr [rsp + 16]",
        "mov r12, qword ptr [rsp + 24]",
        "mov r11, qword ptr [rsp + 32]", // rflags
        "mov r10, qword ptr [rsp + 40]",
        "mov r9,  qword ptr [rsp + 48]",
        "mov r8,  qword ptr [rsp + 56]",
        "mov rsi, qword ptr [rsp + 64]",
        "mov rdi, qword ptr [rsp + 72]",
        "mov rbp, qword ptr [rsp + 80]",
        "mov rdx, qword ptr [rsp + 88]",
        "mov rcx, qword ptr [rsp + 96]", // rip
        "mov rbx, qword ptr [rsp + 104]",
        "mov rax, qword ptr [rsp + 112]",
        "mov rsp, qword ptr [rsp + 120]", // user rsp
        "swapgs",
        "sysretq",
    );
}
