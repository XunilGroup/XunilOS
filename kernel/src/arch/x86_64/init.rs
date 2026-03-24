use crate::arch::x86_64::gdt::load_gdt_x86_64;
use crate::arch::x86_64::interrupts::{PICS, init_idt_x86_64};
use x86_64::instructions::interrupts;

pub fn init_x86_64() {
    load_gdt_x86_64();
    init_idt_x86_64();

    unsafe {
        let mut pics = PICS.lock();
        pics.initialize();
        pics.write_masks(0xFC, 0xFF);
    }

    interrupts::enable();
}
