use core::arch::asm;

use x86_64::instructions::tlb::flush_all;

use crate::{
    arch::arch::safe_lock,
    arch::x86_64::gdt::{GDT, TSS_MUTEX},
    task::context::UserContext,
    task::process::ProcessState,
    task::scheduler::{SCHEDULER, current_pid},
};

const IA32_EFER: u32 = 0xC0000080;
const IA32_EFER_SCE: u64 = 1; // system call enable bit
const IA32_STAR: u32 = 0xC0000081;
const IA32_LSTAR: u32 = 0xC0000082;
const IA32_FMASK: u32 = 0xC0000084;
const IA32_KERNEL_GS_BASE: u32 = 0xC0000102;
const IA32_GS_BASE: u32 = 0xC0000101;

#[repr(C)]
pub struct PerCpuData {
    pub user_rsp: u64,
    pub kernel_rsp: u64,
}

pub static mut PER_CPU: PerCpuData = PerCpuData {
    user_rsp: 0,
    kernel_rsp: 0,
};

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
        );
    }
}

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            lateout("eax") low,
            lateout("edx") high,
        );
    }
    ((high as u64) << 32) | (low as u64)
}

pub fn init_syscalls() {
    unsafe {
        flush_all();

        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | IA32_EFER_SCE); // enable syscalls

        let per_cpu_addr = &raw const PER_CPU as u64;
        wrmsr(IA32_KERNEL_GS_BASE, per_cpu_addr); // per-cpu kernel data

        asm!("mov gs, ax", in("ax") 0u16);

        if let Some(tss) = TSS_MUTEX.lock().as_ref() {
            PER_CPU.kernel_rsp = tss.privilege_stack_table[0].as_u64();
        }

        #[allow(function_casts_as_integer)]
        wrmsr(IA32_LSTAR, syscall_entry as u64); // set syscall entry function

        wrmsr(IA32_FMASK, 1 << 9); // Mask interupts during syscalls

        wrmsr(IA32_GS_BASE, 0);

        let kernel_cs = (GDT.1.0.0 & !0b11) as u64;
        let user_ds = (GDT.1.3.0 & !0b11) as u64;

        let sysret_base = user_ds - 8;

        let star = (kernel_cs << 32) | (sysret_base << 48);
        wrmsr(IA32_STAR, star);
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        r#"
            swapgs
            mov gs:[0], rsp
            mov rsp, gs:[8]

            sub rsp, 144

            mov qword ptr [rsp + 0],   r15
            mov qword ptr [rsp + 8],   r14
            mov qword ptr [rsp + 16],  r13
            mov qword ptr [rsp + 24],  r12
            mov qword ptr [rsp + 32],  r11
            mov qword ptr [rsp + 40],  r10
            mov qword ptr [rsp + 48],  r9
            mov qword ptr [rsp + 56],  r8
            mov qword ptr [rsp + 64],  rsi
            mov qword ptr [rsp + 72],  rdi
            mov qword ptr [rsp + 80],  rbp
            mov qword ptr [rsp + 88],  rdx
            mov qword ptr [rsp + 96],  rcx
            mov qword ptr [rsp + 104], rbx
            mov qword ptr [rsp + 112], rax

            mov rax, qword ptr gs:[0]
            mov qword ptr [rsp + 120], rax

            mov rax, qword ptr [rsp + 96]
            mov qword ptr [rsp + 128], rax
            mov rax, qword ptr [rsp + 32]
            mov qword ptr [rsp + 136], rax

            mov rbx, rsp  // rbx = frame base

            // ctx_save(frame)
            mov rdi, rbx
            lea rsp, [rbx - 8]
            call ctx_save
            mov rsp, rbx

            sti

            mov rdi, qword ptr [rbx + 112]
            mov rsi, qword ptr [rbx + 72]
            mov rdx, qword ptr [rbx + 64]
            mov rcx, qword ptr [rbx + 88]
            mov r8,  qword ptr [rbx + 40]
            mov r9,  qword ptr [rbx + 56]

            lea rsp, [rbx - 24]
            mov rax, qword ptr [rbx + 48]
            mov qword ptr [rsp + 0], rax
            call syscall_dispatch
            mov rsp, rbx

            mov qword ptr [rbx + 112], rax

            mov rdi, rbx
            lea rsp, [rbx - 8]
            call ctx_save
            mov rsp, rbx

            mov rdi, rbx
            call syscall_yield_check

            mov rax, qword ptr [rsp + 128]
            mov qword ptr [rsp + 96], rax
            mov rax, qword ptr [rsp + 136]
            mov qword ptr [rsp + 32], rax

            // restore registers from frame
            mov rax, qword ptr [rsp + 112]
            mov rbx, qword ptr [rsp + 104]
            mov rcx, qword ptr [rsp + 96]
            mov rdx, qword ptr [rsp + 88]
            mov rbp, qword ptr [rsp + 80]
            mov rdi, qword ptr [rsp + 72]
            mov rsi, qword ptr [rsp + 64]
            mov r8,  qword ptr [rsp + 56]
            mov r9,  qword ptr [rsp + 48]
            mov r10, qword ptr [rsp + 40]
            mov r11, qword ptr [rsp + 32]
            mov r12, qword ptr [rsp + 24]
            mov r13, qword ptr [rsp + 16]
            mov r14, qword ptr [rsp + 8]
            mov r15, qword ptr [rsp + 0]
            cli
            mov rsp, qword ptr gs:[0]
            swapgs
            sysretq
        .done:
            ud2
        "#,
    );
}

#[unsafe(no_mangle)]
unsafe extern "C" fn syscall_yield_check(_ctx: *mut UserContext) {
    if let Some(pid) = current_pid() {
        let needs_yield = {
            let guard = safe_lock(|| SCHEDULER.lock());
            guard
                .processes
                .get(&pid)
                .map_or(false, |p| !matches!(p.state, ProcessState::Running))
        };
        if needs_yield {
            SCHEDULER.switch_next(true);
        }
    }
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe fn run_next(ctx: *const UserContext, user_rsp: u64, user_cs: u64, user_ss: u64) {
    core::arch::naked_asm!(
        // rdi = ctx, rsi = user_rsp, rdx = user_cs, rcx = user_ss
        "mov gs:[0], rsi",
        "mov rax, rcx",
        "and eax, 0xFFFC",
        "mov ds, ax",
        "mov es, ax",
        // save ctx ptr on stack, then build iret frame
        "push rdi",
        "push rcx",
        "push rsi",
        "push qword ptr [rdi + 136]",
        "push rdx",
        "push qword ptr [rdi + 128]",
        // load all gp regs via rdi
        "mov r15, qword ptr [rdi + 0]",
        "mov r14, qword ptr [rdi + 8]",
        "mov r13, qword ptr [rdi + 16]",
        "mov r12, qword ptr [rdi + 24]",
        "mov r11, qword ptr [rdi + 32]",
        "mov r10, qword ptr [rdi + 40]",
        "mov r9,  qword ptr [rdi + 48]",
        "mov r8,  qword ptr [rdi + 56]",
        "mov rsi, qword ptr [rdi + 64]",
        "mov rbp, qword ptr [rdi + 80]",
        "mov rdx, qword ptr [rdi + 88]",
        "mov rcx, qword ptr [rdi + 96]",
        // ctx_ptr is at [rsp + 40]
        "mov rax, qword ptr [rsp + 40]",
        "mov rbx, qword ptr [rax + 104]",
        "mov rdi, qword ptr [rax + 72]",
        "mov rax, qword ptr [rax + 112]",
        "swapgs",
        "iretq",
    );
}
