use crate::{
    arch::aarch64::{
        heap::init_heap,
        interrupts::init_interrupts,
        kmi::setup_kmi,
        paging::{AArchPageTable, initialize_paging_aarch64},
    },
    config::TIMER_FREQUENCY_HZ,
};
use limine::response::{ExecutableAddressResponse, HhdmResponse, MemoryMapResponse};

// needs to be page aligned since we map it into the page table
#[repr(align(4096))]
pub struct Stack(pub [u8; 64 * 1024]);

pub static KERNEL_STACK: Stack = Stack([0; 64 * 1024]);

pub fn set_timer_freq(freq: usize) {
    unsafe {
        core::arch::asm!("mrs {}, cntp_tval_el0", in(reg) freq);
    }
}

#[unsafe(naked)]
pub unsafe fn init_aarch64_trampoline(mapper: &mut AArchPageTable) {
    // fix stack, since limine's bootloader stack is not mapped
    core::arch::naked_asm!(
        "msr spsel, #1",
        "adrp x1, {stack}",
        "add x1, x1, :lo12:{stack}",
        "add sp, x1, {size}",
        "b kernel_main_aarch64",
        stack = sym KERNEL_STACK,
        size = const 64 * 1024,
    );
}

#[unsafe(no_mangle)]
pub extern "C" fn init_aarch64(mapper: &mut AArchPageTable) {
    init_heap(mapper);
    set_timer_freq(TIMER_FREQUENCY_HZ);
    init_interrupts();
    unsafe { setup_kmi() };
}

pub fn preinit_aarch64<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
    executable_address_response: &ExecutableAddressResponse,
) {
    let mut mapper = initialize_paging_aarch64(
        hhdm_response,
        memory_map_response,
        executable_address_response,
    );
    unsafe { init_aarch64_trampoline(&mut mapper) };
}
