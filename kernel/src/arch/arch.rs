#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::paging::FRAME_ALLOCATOR_X86_64 as FRAME_ALLOCATOR;

use crate::{driver::timer::TIMER, util::serial_print};
use alloc::string::ToString;
use core::{alloc::GlobalAlloc, arch::asm, sync::atomic::Ordering};
use limine::response::{HhdmResponse, MemoryMapResponse};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{
    elf::run_elf_x86_64,
    heap::ALLOCATOR,
    init::init_x86_64,
    paging::{FRAME_ALLOCATOR_X86_64, XunilFrameAllocator},
    usermode::enter_usermode_x86_64,
};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::OffsetPageTable;

#[cfg(target_arch = "x86_64")]
pub fn init<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
) -> (OffsetPageTable<'static>) {
    return init_x86_64(hhdm_response, memory_map_response);
}

#[cfg(target_arch = "x86_64")]
pub fn enter_usermode(user_rip: u64, user_rsp: u64) {
    return enter_usermode_x86_64(user_rip, user_rsp);
}

#[cfg(target_arch = "x86_64")]
pub fn run_elf(entry_point: *const u8, heap_base: u64) {
    run_elf_x86_64(entry_point, heap_base);
}

pub fn get_allocator<'a>() -> &'static impl GlobalAlloc {
    return &ALLOCATOR;
}

pub fn idle() {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!("hlt");
        #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
        asm!("wfi");
        #[cfg(target_arch = "loongarch64")]
        asm!("idle 0");
    }
}

pub fn sleep(ticks: u64) {
    // let start = TIMER.now();
    // while start.ticks_since() < ticks {
    //     serial_print(start.ticks_since().to_string().as_str());
    //     core::hint::spin_loop();
    // }
}

pub fn infinite_idle() -> ! {
    loop {
        idle()
    }
}

#[inline(always)]
pub fn kernel_crash() -> ! {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("ud2")
    };

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("udf #0")
    };

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("unimp")
    };

    loop {} // satisfies -> ! on unknown archs
}
