use crate::task::scheduler::{SCHEDULER, current_pid};

#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserContext {
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64,
    pub x30: u64,
    pub elr_el1: u64,
    pub spsr_el1: u64,
    pub esr_el1: u64, // exception type
    pub far_el1: u64, // fault type
    pub sp_el0: u64,
    pub sp_el1: u64,
    pub _pad1: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn ctx_save(regs: *const UserContext) {
    if let Some(pid) = current_pid() {
        let mut guard = SCHEDULER.lock();
        if let Some(process) = guard.processes.get_mut(&pid) {
            let saved_ctx = unsafe { core::ptr::read_unaligned(regs) };

            process.saved_ctx = Some(saved_ctx);
        }
    }
}
