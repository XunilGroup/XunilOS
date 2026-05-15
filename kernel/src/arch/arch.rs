#![allow(dead_code, unused_imports)]
#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::paging::FRAME_ALLOCATOR_X86_64 as FRAME_ALLOCATOR;
#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::{init::init_x86_64, usermode::enter_usermode_x86_64};
#[cfg(target_arch = "aarch64")]
use limine::response::ExecutableAddressResponse;
#[cfg(target_arch = "x86_64")]
use x86_64::{
    instructions::interrupts::without_interrupts,
    structures::paging::{FrameAllocator, OffsetPageTable, PhysFrame, Size4KiB},
};

#[cfg(target_arch = "aarch64")]
pub use crate::arch::aarch64::paging::FRAME_ALLOCATOR_AARCH64 as FRAME_ALLOCATOR;
#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::{
    init::init_aarch64, paging::AArchPageTable, usermode::enter_usermode_aarch64,
};

use crate::{driver::timer::TIMER, util::align_up};
use core::{arch::asm, sync::atomic::AtomicU64};
use limine::{
    memory_map::{Entry, EntryType},
    response::{HhdmResponse, MemoryMapResponse},
};

#[cfg(target_arch = "aarch64")]
const UART: *mut u8 = 0x0900_0000 as *mut u8;

pub static HHDM_OFFSET: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct UsableRegion {
    pub base: u64,
    pub length: u64,
}

const EMPTY_REGION: UsableRegion = UsableRegion { base: 0, length: 0 };

// NOTE: dont change name to frameallocator as that is the name of the trait of x86_64
pub struct XunilFrameAllocator {
    pub hhdm_offset: u64,
    pub usable_regions: [UsableRegion; 1024],
    pub usable_region_count: usize,
    pub region_index: usize,
    pub region_offset: usize,
}

impl XunilFrameAllocator {
    pub const fn new() -> Self {
        Self {
            hhdm_offset: 0,
            usable_regions: [EMPTY_REGION; 1024],
            usable_region_count: 0,
            region_index: 0,
            region_offset: 0,
        }
    }

    pub fn initialize(&mut self, memory_map: &[&Entry]) {
        let mut regions = [EMPTY_REGION; 1024];
        let mut count = 0usize;

        for region in memory_map.iter().copied() {
            if region.entry_type != EntryType::USABLE {
                continue;
            }

            if count < regions.len() && region.length >= 4096 {
                let aligned_base = align_up(region.base, 4096);
                let base_offset = aligned_base - region.base;
                let aligned_length = region.length.saturating_sub(base_offset);
                if aligned_length >= 4096 {
                    regions[count] = UsableRegion {
                        base: aligned_base,
                        length: aligned_length,
                    };
                    count += 1;
                }
            }
        }

        self.hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        self.usable_regions = regions;
        self.usable_region_count = count;
        self.region_index = 0;
        self.region_offset = 0;
    }
}

#[cfg(target_arch = "x86_64")]
pub fn init<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
) -> OffsetPageTable<'static> {
    return init_x86_64(hhdm_response, memory_map_response);
}

#[cfg(target_arch = "aarch64")]
pub fn init<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
    executable_address_response: &ExecutableAddressResponse,
) -> AArchPageTable {
    return init_aarch64(
        hhdm_response,
        memory_map_response,
        executable_address_response,
    );
}

#[cfg(target_arch = "x86_64")]
pub fn enter_usermode(entry: u64, stack_ptr: u64, should_swapgs: bool) {
    enter_usermode_x86_64(entry, stack_ptr, should_swapgs);
}

#[cfg(target_arch = "aarch64")]
pub fn enter_usermode(entry: u64, stack_ptr: u64, should_swapgs: bool) {
    enter_usermode_aarch64(entry, stack_ptr, should_swapgs);
}

pub fn safe_lock<R, F: FnOnce() -> R>(f: F) -> R {
    #[cfg(target_arch = "x86_64")]
    return without_interrupts(|| f());
    #[cfg(target_arch = "aarch64")]
    return f();
}

#[cfg(target_arch = "x86_64")]
pub fn serial_print_byte(b: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") b);
    }
}

#[cfg(target_arch = "aarch64")]
pub fn serial_print_byte(b: u8) {
    unsafe {
        let buf = [b];
        core::arch::asm!(
            "hlt #0xF000",
            in("x0") 0x03u64,  // SYS_WRITEC
            in("x1") buf.as_ptr(),
            options(nostack)
        );
    }
}

pub fn serial_print(s: &str) {
    for &b in s.as_bytes() {
        serial_print_byte(b);
    }
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
    while start.ticks_since() < ticks {
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
