use core::arch::asm;

use x86_64::instructions::tlb::flush_all;

use crate::{
    arch::x86_64::gdt::{GDT, TSS},
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

static mut PER_CPU: PerCpuData = PerCpuData {
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

        let kernel_stack_top = TSS.privilege_stack_table[0].as_u64();
        PER_CPU.kernel_rsp = kernel_stack_top;

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

#[unsafe(no_mangle)]
unsafe extern "C" fn check_and_reschedule() -> usize {
    let pid = current_pid().unwrap_or(0);

    if pid == 0 {
        return 0;
    }

    let should = SCHEDULER
        .with_process(pid, |process| process.should_reschedule)
        .unwrap_or(false);

    if !should {
        return 0;
    }

    let next_task = SCHEDULER.next_task();

    if next_task == pid {
        return 0;
    }

    SCHEDULER.switch_to(next_task, false);

    1
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
unsafe extern "C" fn syscall_entry() {
    core::arch::naked_asm!(
        r#"
            swapgs
            mov gs:[0], rsp
            mov rsp, gs:[8]

            sub rsp, 128

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

            mov rbx, rsp  // rbx = frame base

            // ctx_save(frame)
            mov rdi, rbx
            lea rsp, [rbx - 8]
            call ctx_save
            mov rsp, rbx

            // syscall_dispatch(num,arg0..arg5)
            mov rdi, qword ptr [rbx + 112]
            mov rsi, qword ptr [rbx + 72]
            mov rdx, qword ptr [rbx + 64]
            mov rcx, qword ptr [rbx + 88]
            mov r8,  qword ptr [rbx + 40]
            mov r9,  qword ptr [rbx + 56]

            lea rsp, [rbx - 24]              // rsp%16==8 before call
            mov rax, qword ptr [rbx + 48]
            mov qword ptr [rsp + 0], rax
            call syscall_dispatch
            mov rsp, rbx

            // save return value into frame (so normal restore uses it)
            mov qword ptr [rbx + 112], rax

            // ctx_save(frame) again so saved_ctx.rax reflects return value
            mov rdi, rbx
            lea rsp, [rbx - 8]
            call ctx_save
            mov rsp, rbx

            // maybe switch tasks (this should not return if it switches)
            lea rsp, [rbx - 8]
            call check_and_reschedule
            mov rsp, rbx

            test rax, rax
            jnz .done

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
            mov r11, qword ptr [rsp + 32]    // rflags — sysretq loads RFLAGS from r11
            mov r12, qword ptr [rsp + 24]
            mov r13, qword ptr [rsp + 16]
            mov r14, qword ptr [rsp + 8]
            mov r15, qword ptr [rsp + 0]
            // don't restore rsp from frame — use the user rsp we saved in gs:[0]
            mov rsp, qword ptr gs:[0]
            swapgs
            sysretq
        .done:
            ud2
        "#,
    );
}
