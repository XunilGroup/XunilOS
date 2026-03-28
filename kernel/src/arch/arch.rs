use crate::driver::timer::TIMER;
use core::arch::asm;
use limine::response::{HhdmResponse, MemoryMapResponse};

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{init::init_x86_64, paging::XunilFrameAllocator};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::OffsetPageTable;

#[cfg(target_arch = "x86_64")]
pub fn init<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
) -> (OffsetPageTable<'static>, XunilFrameAllocator<'a>) {
    return init_x86_64(hhdm_response, memory_map_response);
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
    let start = TIMER.now();
    while (TIMER.now() - start).elapsed() <= ticks {
        idle();
    }
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
