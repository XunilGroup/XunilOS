use core::sync::atomic::{AtomicU64, Ordering};

use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    arch::arch::{GLOBAL_TICK_COUNT, safe_lock},
    config::TIMER_FREQUENCY_HZ,
    driver::timer::TIMER,
    task::{
        context::UserContext,
        process::{Process, ProcessState},
    },
    util::Locked,
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::interrupts::run_next;
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{
    gdt::{user_code_selector, user_data_selector},
    syscall::run_next,
    usermode::enter_usermode_x86_64,
};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::gdt::SegmentSelector;

#[cfg(target_arch = "x86_64")]
fn with_rpl3(ss: SegmentSelector) -> u64 {
    (ss.0 as u64) | 3
}

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
    pub fn spawn_process(
        &self,
        entry_point: u64,
        stack_top: u64,
        kernel_stack_top: u64,
        heap_base: u64,
    ) -> Option<u64> {
        let mut guard = safe_lock(|| self.lock());
        let pid = guard.next_pid;
        guard.next_pid += 1;
        let process = Process::new(
            pid,
            entry_point,
            stack_top,
            kernel_stack_top,
            heap_base,
            heap_base,
        );
        guard.processes.insert(pid, process);

        Some(pid)
    }

    pub fn next_task(&self) -> u64 {
        if let Some(previous_pid) = current_pid() {
            let mut guard = safe_lock(|| self.lock());

            for process in guard.processes.values_mut() {
                if process.info.wake_tick.is_some() {
                    if TIMER.now().elapsed() >= process.info.wake_tick.unwrap() {
                        process.state = ProcessState::Ready;
                        process.info.wake_tick = None;
                    }
                }
            }

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

    #[allow(unused_variables)]
    pub fn switch_to(&self, pid: u64, should_swapgs: bool) {
        let (ctx_opt, entry, stack_top, kernel_stack_top) = {
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
                new_process.kernel_stack_top,
            )
        };

        set_current_pid(Some(pid));

        #[cfg(target_arch = "x86_64")]
        unsafe {
            use x86_64::VirtAddr;

            use crate::arch::x86_64::{gdt::TSS_MUTEX, syscall::PER_CPU};

            PER_CPU.kernel_rsp = kernel_stack_top;
            if let Some(tss) = TSS_MUTEX.lock().as_mut() {
                tss.privilege_stack_table[0] = VirtAddr::new(kernel_stack_top);
            }
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let saved_ctx = ctx_opt.expect("Could not get user context");
            run_next((&saved_ctx) as *const UserContext, saved_ctx.sp_el0);
        }

        #[cfg(target_arch = "x86_64")]
        match ctx_opt {
            Some(saved_ctx) => unsafe {
                let user_cs = with_rpl3(user_code_selector());
                let user_ss = with_rpl3(user_data_selector());
                run_next(
                    (&saved_ctx) as *const UserContext,
                    saved_ctx.rsp,
                    user_cs,
                    user_ss,
                );
            },
            None => {
                enter_usermode_x86_64(entry as u64, (stack_top & !0xF) - 8, should_swapgs);
            }
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

#[unsafe(no_mangle)]
pub extern "C" fn check_and_reschedule() -> isize {
    let current_pid = CURRENT_PID.load(Ordering::Relaxed);
    let should = safe_lock(|| {
        let mut scheduler = SCHEDULER.lock();

        if let Some(process) = scheduler.processes.get_mut(&current_pid) {
            let elapsed = GLOBAL_TICK_COUNT.load(Ordering::Relaxed) - process.last_switch_tick;
            if elapsed >= (TIMER_FREQUENCY_HZ / 60) as u64 {
                process.last_switch_tick = GLOBAL_TICK_COUNT.load(Ordering::Relaxed);
                true
            } else {
                false
            }
        } else {
            false
        }
    });

    if should {
        let next_task = SCHEDULER.next_task();

        if next_task == current_pid {
            return 0;
        }

        SCHEDULER.switch_to(next_task, true);

        1
    } else {
        0
    }
}
