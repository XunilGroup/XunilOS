use core::arch::asm;

use x86_64::instructions::tlb::flush_all;

use crate::arch::x86_64::gdt::{GDT, TSS};

const IA32_EFER: u32 = 0xC0000080;
const IA32_EFER_SCE: u64 = 1; // system call enable bit
const IA32_STAR: u32 = 0xC0000081;
const IA32_LSTAR: u32 = 0xC0000082;
const IA32_FMASK: u32 = 0xC0000084;
const IA32_KERNEL_GS_BASE: u32 = 0xC0000102;

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
            .intel_syntax noprefix

            swapgs

            // save user stack
            mov gs:[0], rsp

            // use kernel stack
            mov rsp, gs:[8]

            push r11
            push rcx
            push rax
            push rdi
            push rsi
            push rdx
            push r10
            push r8
            push r9

            mov rdi, rax
            mov rsi, [rsp + 40]
            mov rdx, [rsp + 32]
            mov rcx, [rsp + 24]
            mov r8, [rsp + 16]
            mov r9, [rsp + 8]
            push qword ptr [rsp]

            call syscall_dispatch

            add rsp, 8 // fix alignment

            pop r9
            pop r8
            pop r10
            pop rdx
            pop rsi
            pop rdi
            add rsp, 8

            pop rcx
            pop r11

            mov rsp, gs:[0]

            swapgs
            sysretq"#,
    );
}
