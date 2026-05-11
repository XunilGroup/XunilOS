use crate::task::scheduler::{SCHEDULER, current_pid};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserContext {
    //general purpose data regs
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64, // rflags
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64, // rip
    pub rbx: u64,
    pub rax: u64,
    pub rsp: u64, // user rsp
}

#[unsafe(no_mangle)]
pub extern "C" fn ctx_save(regs: *const UserContext) {
    if let Some(pid) = current_pid() {
        let mut guard = SCHEDULER.lock();
        if let Some(process) = guard.processes.get_mut(&pid) {
            let saved_ctx = unsafe { core::ptr::read(regs) };
            process.saved_ctx = Some(saved_ctx);
        }
    }
}
