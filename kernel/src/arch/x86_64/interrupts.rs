use crate::{
    arch::x86_64::{gdt, mouse::mouse_interrupt},
    driver::{
        keyboard::{KEYBOARD_STATE, KeyboardEvent},
        mouse::MOUSE,
        timer::TIMER,
    },
    println,
    util::serial_print,
};
use lazy_static::lazy_static;
use pc_keyboard::DecodedKey;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    VirtAddr,
    instructions::port::Port,
    registers::control::Cr2,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

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
            #[allow(function_casts_as_integer)]
            idt[0x80]
                .set_handler_addr(VirtAddr::new(syscall_interrupt_handler as u64))
                .set_privilege_level(x86_64::PrivilegeLevel::Ring3);
        }
        idt[InterruptIndex::Timer.as_u8()].set_handler_fn(timer_interrupt_handler);
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
        "EXCEPTION: PAGE FAULT\nAccessed Addresss: {:?}\nError Code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
}

pub extern "x86-interrupt" fn gpf_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT\nError Code: {:?}\n{:#?}",
        error_code, stack_frame
    );
}

pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: INVALID OPCODE\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    TIMER.interrupt();

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let interrupt_result = mouse_interrupt();

    if let Some(interrupt_result) = interrupt_result {
        MOUSE.interrupt(
            interrupt_result.0,
            interrupt_result.1,
            interrupt_result.2,
            interrupt_result.3,
            interrupt_result.4,
        );
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    let mut keyboard_state = KEYBOARD_STATE.lock();

    if let Ok(Some(key_event)) = keyboard_state.keyboard.add_byte(scancode) {
        if let Some(key) = keyboard_state.keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => keyboard_state
                    .event_queue
                    .push_back(KeyboardEvent::Unicode(character)),
                DecodedKey::RawKey(key) => keyboard_state
                    .event_queue
                    .push_back(KeyboardEvent::RawKey(key)),
            }
        }
    }

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[unsafe(naked)]
unsafe extern "C" fn syscall_interrupt_handler() {
    core::arch::naked_asm!(
        // push all registers
        "push r15",
        "push r14",
        "push r13",
        "push r12",
        "push r11",
        "push r10",
        "push r9",
        "push r8",
        "push rbp",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rax",
        "sub rsp, 8",
        "mov rcx, rdx", // arg2
        "mov rdx, rsi", // arg1
        "mov rsi, rdi", // arg0
        "mov rdi, rax", // num
        "call syscall_dispatch",
        "add rsp, 8",
        "add rsp, 8",
        // pop them in reverse orser
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rbp",
        "pop r8",
        "pop r9",
        "pop r10",
        "pop r11",
        "pop r12",
        "pop r13",
        "pop r14",
        "pop r15",
        "iretq",
    )
}
