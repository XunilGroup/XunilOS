use core::sync::atomic::{AtomicU64, Ordering};

use crate::{
    arch::arch::{GLOBAL_TICK_COUNT, safe_lock},
    driver::timer::TIMER,
    task::{
        context::UserContext,
        process::{Process, ProcessState},
    },
    util::Locked,
};
use alloc::collections::{binary_heap::BinaryHeap, btree_map::BTreeMap, vec_deque::VecDeque};
use core::cmp::Reverse;

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::interrupts::run_next;
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{
    gdt::{user_code_selector, user_data_selector},
    syscall::run_next,
    usermode::enter_usermode_x86_64,
};
use crate::task::context::ctx_save;
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

enum SwitchDecision {
    Switch(Option<UserContext>, u64, u64, u64),
    Stay,
    Idle,
}

pub struct Scheduler {
    pub processes: BTreeMap<u64, Process>,
    ready_queue: VecDeque<u64>,
    pub sleep_queue: BinaryHeap<Reverse<(u64, u64)>>,
    next_pid: u64,
}

impl Scheduler {
    pub const fn new() -> Scheduler {
        Scheduler {
            processes: BTreeMap::new(),
            ready_queue: VecDeque::new(),
            sleep_queue: BinaryHeap::new(),
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

        if let Some(new_process) = guard.processes.get_mut(&pid) {
            new_process.in_ready_queue = true;
        }
        guard.ready_queue.push_back(pid);

        Some(pid)
    }

    pub fn switch_next(&self, should_swapgs: bool) -> bool {
        let previous_pid = current_pid().unwrap_or(0);
        if previous_pid == 0 {
            return false;
        }

        loop {
            let decision = safe_lock(|| {
                let mut guard = self.lock();
                self.wake_processes(&mut guard);
                self.mark_old_process_ready(&mut guard);

                let mut chosen: Option<u64> = None;
                while let Some(pid) = guard.ready_queue.pop_front() {
                    if let Some(process) = guard.processes.get_mut(&pid) {
                        process.in_ready_queue = false;
                        if matches!(process.state, ProcessState::Ready) {
                            chosen = Some(pid);
                            break;
                        }
                    };
                }

                let new_pid = match chosen {
                    Some(pid) => pid,
                    None => {
                        let is_prev_ready = matches!(
                            guard
                                .processes
                                .get(&previous_pid)
                                .map(|process| &process.state),
                            Some(ProcessState::Ready)
                        );

                        if is_prev_ready {
                            previous_pid
                        } else {
                            return SwitchDecision::Idle;
                        }
                    }
                };

                if Some(new_pid) == current_pid() || new_pid == previous_pid {
                    if let Some(p) = guard.processes.get_mut(&new_pid) {
                        p.state = ProcessState::Running;
                        p.last_switch_tick = GLOBAL_TICK_COUNT.load(Ordering::Relaxed);
                    }

                    return SwitchDecision::Stay;
                }

                let new_process = guard
                    .processes
                    .get_mut(&new_pid)
                    .expect("Cant get new process");

                new_process.state = ProcessState::Running;
                new_process.last_switch_tick = GLOBAL_TICK_COUNT.load(Ordering::Relaxed);
                new_process.in_ready_queue = false;
                if let Some(address_space) = new_process.address_space.as_mut() {
                    address_space.use_address_space();
                }

                let ctx_opt = new_process.saved_ctx;
                let entry = new_process.user_entry;
                let stack_top = new_process.stack_top;
                let kernel_stack_top = new_process.kernel_stack_top;

                set_current_pid(Some(new_pid));
                #[cfg(target_arch = "x86_64")]
                unsafe {
                    use crate::arch::x86_64::{gdt::TSS_MUTEX, syscall::PER_CPU};
                    use x86_64::VirtAddr;

                    PER_CPU.kernel_rsp = kernel_stack_top;
                    if let Some(tss) = TSS_MUTEX.lock().as_mut() {
                        tss.privilege_stack_table[0] = VirtAddr::new(kernel_stack_top);
                    }
                }

                #[cfg(target_arch = "aarch64")]
                let _ = kernel_stack_top;

                SwitchDecision::Switch(ctx_opt, entry, stack_top, new_pid)
            });

            match decision {
                SwitchDecision::Switch(ctx_opt, entry, stack_top, _new_pid) => {
                    self.do_context_switch(ctx_opt, entry, stack_top, should_swapgs);
                    return true;
                }

                SwitchDecision::Stay => {
                    return false;
                }

                SwitchDecision::Idle => {
                    #[cfg(target_arch = "x86_64")]
                    {
                        x86_64::instructions::interrupts::enable_and_hlt();
                        x86_64::instructions::interrupts::disable();
                    }

                    #[cfg(target_arch = "aarch64")]
                    unsafe {
                        use aarch64_cpu::registers::{DAIF, Writeable};
                        DAIF.write(
                            DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Unmasked + DAIF::F::Masked,
                        );
                        core::arch::asm!("wfi");
                        DAIF.write(
                            DAIF::D::Masked + DAIF::A::Masked + DAIF::I::Masked + DAIF::F::Masked,
                        );
                    }
                }
            }
        }
    }

    pub fn wake_processes(&self, guard: &mut Scheduler) {
        let now = TIMER.now().elapsed();
        while let Some(&Reverse((wake_tick, pid))) = guard.sleep_queue.peek() {
            if wake_tick > now {
                break;
            }

            guard.sleep_queue.pop();

            if let Some(process) = guard.processes.get_mut(&pid) {
                if process.info.wake_tick == Some(wake_tick) {
                    process.state = ProcessState::Ready;
                    process.info.wake_tick = None;
                    if !process.in_ready_queue {
                        process.in_ready_queue = true;
                        guard.ready_queue.push_back(process.pid);
                    }
                }
            }
        }
    }

    #[allow(unused_variables)]
    fn do_context_switch(
        &self,
        ctx_opt: Option<UserContext>,
        entry: u64,
        stack_top: u64,
        should_swapgs: bool,
    ) {
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

    pub fn switch_to(&self, pid: u64, should_swapgs: bool) {
        let switch_info = safe_lock(|| {
            let mut guard = self.lock();
            self.wake_processes(&mut guard);
            self.mark_old_process_ready(&mut guard);
            let mut new_process = guard.processes.get_mut(&pid).expect("Cant get new process");
            let (ctx_opt, entry, stack_top, kernel_stack_top) = self.use_process(&mut new_process);

            set_current_pid(Some(pid));
            #[cfg(target_arch = "x86_64")]
            unsafe {
                use crate::arch::x86_64::{gdt::TSS_MUTEX, syscall::PER_CPU};
                use x86_64::VirtAddr;

                PER_CPU.kernel_rsp = kernel_stack_top;
                if let Some(tss) = TSS_MUTEX.lock().as_mut() {
                    tss.privilege_stack_table[0] = VirtAddr::new(kernel_stack_top);
                }
            }

            Some((ctx_opt, entry, stack_top))
        });

        if let Some((ctx_opt, entry, stack_top)) = switch_info {
            self.do_context_switch(ctx_opt, entry, stack_top, should_swapgs);
        }
    }

    #[inline]
    pub fn use_process(&self, new_process: &mut Process) -> (Option<UserContext>, u64, u64, u64) {
        new_process.last_switch_tick = GLOBAL_TICK_COUNT.load(Ordering::Relaxed);
        new_process.state = ProcessState::Running;
        new_process.in_ready_queue = false;
        if let Some(address_space) = new_process.address_space.as_mut() {
            address_space.use_address_space();
        };

        (
            new_process.saved_ctx,
            new_process.user_entry,
            new_process.stack_top,
            new_process.kernel_stack_top,
        )
    }

    pub fn mark_old_process_ready(&self, guard: &mut Scheduler) {
        if let Some(previous_pid) = current_pid() {
            if let Some(old_process) = guard.processes.get_mut(&previous_pid) {
                if matches!(old_process.state, ProcessState::Running) && !old_process.in_ready_queue
                {
                    old_process.state = ProcessState::Ready;
                    old_process.in_ready_queue = true;
                    guard.ready_queue.push_back(old_process.pid);
                }
            }
        }
    }

    pub fn terminate_process(&self, pid: u64, exit_code: isize) {
        safe_lock(|| {
            let mut guard = self.lock();
            if let Some(process) = guard.processes.get_mut(&pid) {
                process.state = ProcessState::Zombie;
                process.in_ready_queue = false;
                process.input_buffer.clear();
                process.info.exit_code = exit_code;
            }
        });
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
pub extern "C" fn check_and_reschedule(ctx: &*mut UserContext) -> isize {
    let current_pid = CURRENT_PID.load(Ordering::Relaxed);
    let should = safe_lock(|| {
        let mut scheduler = SCHEDULER.lock();

        SCHEDULER.wake_processes(&mut scheduler);

        if let Some(process) = scheduler.processes.get_mut(&current_pid) {
            let elapsed = GLOBAL_TICK_COUNT.load(Ordering::Relaxed) - process.last_switch_tick;
            elapsed >= 100
        } else {
            false
        }
    });

    if should {
        ctx_save(*ctx);
        match SCHEDULER.switch_next(true) {
            true => return 1,
            false => return 0,
        }
    }

    0
}
