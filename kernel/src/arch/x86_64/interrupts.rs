use core::arch::naked_asm;

use crate::{
    arch::{arch::do_interrupt, x86_64::gdt},
    driver::io::ps2::{keyboard_interrupt, mouse_interrupt, push_scancode},
    println,
    task::{
        context::UserContext,
        scheduler::{check_and_reschedule, current_pid},
    },
};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    VirtAddr,
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

pub const PIC_1_OFFSET: u8 = 32; // master
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8; // slave

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Mouse = PIC_2_OFFSET + 4,
    // RTC = PIC_2_OFFSET,
    // ATA_primary = PIC_2_OFFSET + 7
    // ATA_secondary = PIC_2_OFFSET + 8
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
            idt[InterruptIndex::Timer.as_u8()]
                .set_handler_addr(VirtAddr::new(timer_interrupt_handler as *const u8 as u64))
                .set_stack_index(gdt::TIMER_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(gpf_handler);
        idt[InterruptIndex::Keyboard.as_u8()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.as_u8()].set_handler_fn(mouse_interrupt_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt
    };
}

pub fn init_idt_x86_64() {
    IDT.load();
}

pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    panic!(
        "EXCEPTION: PAGE FAULT\nAccessed Address: {:?}\nError Code: {:?}\nCurrent PID: {}\nPER_CPU: {:#x}\n{:#?}",
        Cr2::read(),
        error_code,
        current_pid().unwrap_or(0),
        &raw const crate::arch::x86_64::syscall::PER_CPU as u64,
        stack_frame
    );
}

pub extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\nError Code: {:?}\nCurrent PID: {}\n{:#?}",
        error_code,
        current_pid().unwrap_or(0),
        stack_frame
    );
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

#[unsafe(naked)]
pub fn timer_interrupt_handler() {
    naked_asm!(
        r#"
        test qword ptr [rsp + 8], 3
        jz .from_kernel

        swapgs

        sub rsp, 144

        mov [rsp + 0],   r15
        mov [rsp + 8],   r14
        mov [rsp + 16],  r13
        mov [rsp + 24],  r12
        mov [rsp + 32],  r11
        mov [rsp + 40],  r10
        mov [rsp + 48],  r9
        mov [rsp + 56],  r8
        mov [rsp + 64],  rsi
        mov [rsp + 72],  rdi
        mov [rsp + 80],  rbp
        mov [rsp + 88],  rdx
        mov [rsp + 96],  rcx
        mov [rsp + 104], rbx
        mov [rsp + 112], rax

        mov rax, [rsp + 144 + 0]
        mov [rsp + 128], rax
        mov rax, [rsp + 144 + 16]
        mov [rsp + 136], rax
        mov rax, [rsp + 144 + 24]
        mov [rsp + 120], rax

        mov rdi, rsp

        call x86_interrupt
        test rax, rax
        jnz .switched

        mov rax, [rsp + 112]
        mov rbx, [rsp + 104]
        mov rcx, [rsp + 96]
        mov rdx, [rsp + 88]
        mov rbp, [rsp + 80]
        mov rdi, [rsp + 72]
        mov rsi, [rsp + 64]
        mov r8,  [rsp + 56]
        mov r9,  [rsp + 48]
        mov r10, [rsp + 40]
        mov r11, [rsp + 32]
        mov r12, [rsp + 24]
        mov r13, [rsp + 16]
        mov r14, [rsp + 8]
        mov r15, [rsp + 0]

        add rsp, 144
        swapgs
        iretq

        .from_kernel:
        push rbp
        push r15
        push r14
        push r13
        push r12
        push r11
        push r10
        push r9
        push r8
        push rdi
        push rsi
        push rdx
        push rcx
        push rbx
        push rax

        call do_interrupt
        call eoi

        pop rax
        pop rbx
        pop rcx
        pop rdx
        pop rsi
        pop rdi
        pop r8
        pop r9
        pop r10
        pop r11
        pop r12
        pop r13
        pop r14
        pop r15
        pop rbp
        iretq

        .switched:
        ud2
        "#
    )
}

#[unsafe(no_mangle)]
extern "C" fn x86_interrupt(ctx: *mut UserContext) -> isize {
    do_interrupt();
    eoi();
    check_and_reschedule(&ctx)
}

#[unsafe(no_mangle)]
extern "C" fn eoi() {
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    mouse_interrupt();

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    if let Some(scancode) = keyboard_interrupt() {
        push_scancode(scancode);
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
