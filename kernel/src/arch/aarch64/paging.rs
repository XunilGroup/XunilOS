use core::sync::atomic::{AtomicBool, Ordering};

use crate::{
    arch::{
        aarch64::init::KERNEL_STACK,
        arch::{HHDM_OFFSET, XunilFrameAllocator, safe_lock, serial_print},
    },
    util::U64Buf,
};
use limine::{
    memory_map::EntryType,
    response::{ExecutableAddressResponse, HhdmResponse, MemoryMapResponse},
};
use spin::Mutex;

pub static FRAME_ALLOCATOR_AARCH64: Mutex<XunilFrameAllocator> =
    Mutex::new(XunilFrameAllocator::new());

// Constants
const VALID: u64 = 1 << 0;
const TABLE: u64 = 1 << 1;
const PAGE: u64 = 1 << 1;
const AF: u64 = 1 << 10;
const SH_INNER: u64 = 0b11 << 8;

const ATTR_NORMAL: u64 = 0 << 2;
const ATTR_DEVICE: u64 = 1 << 2;

// AP bits
const AP_RW_EL1: u64 = 0b00 << 6; // kernel RW
const AP_RO_EL1: u64 = 0b10 << 6; // kernel RO
const AP_RW_EL0: u64 = 0b01 << 6; // kernel RW, user RW
const AP_RO_EL0: u64 = 0b11 << 6; // kernel RO, user RO

pub const UXN: u64 = 1 << 54; // unprivileged execute never
const PXN: u64 = 1 << 53; // privileged execute never

pub fn kernel_data_flags() -> u64 {
    VALID | PAGE | AF | SH_INNER | AP_RW_EL1 | UXN | ATTR_NORMAL
}

pub fn kernel_code_flags() -> u64 {
    VALID | PAGE | AF | SH_INNER | AP_RO_EL1 | ATTR_NORMAL // no UXN
}

pub fn user_data_flags() -> u64 {
    VALID | PAGE | AF | SH_INNER | AP_RW_EL0 | UXN | PXN | ATTR_NORMAL
}

pub fn user_code_flags() -> u64 {
    VALID | PAGE | AF | SH_INNER | AP_RO_EL0 | ATTR_NORMAL // no UXN
}

pub fn device_flags() -> u64 {
    VALID | PAGE | AF | AP_RW_EL1 | UXN | PXN | ATTR_DEVICE
    // no SH bits for device memory
}

static HHDM_OVERFLOW_REPORTED: AtomicBool = AtomicBool::new(false);

fn phys_to_virt(phys: u64) -> *mut u64 {
    let hhdm_offset = HHDM_OFFSET.load(Ordering::Relaxed);

    match phys.checked_add(hhdm_offset) {
        Some(virt) => virt as *mut u64,
        None => {
            if !HHDM_OVERFLOW_REPORTED.swap(true, Ordering::Relaxed) {
                serial_print("HHDM overflow phys=");
                serial_print(U64Buf::new(phys).as_str());
                serial_print(", hhdm_offset=");
                serial_print(U64Buf::new(hhdm_offset).as_str());
                serial_print("\n");
            }
            panic!("phys_to_virt overflow");
        }
    }
}

pub fn tlb_flush() {
    unsafe {
        core::arch::asm!(
            "tlbi vmalle1is", // invalidate TLB
            "dsb ish",        // wait for tlb flush
            "isb",
        )
    }
}

pub fn set_page_table(root_phys: u64) {
    unsafe {
        core::arch::asm!(
            "msr ttbr1_el1, {root}", // set to new page table
            "isb",
            root = in(reg) root_phys
        )
    }
}

pub fn setup_mair() {
    let mair: u64 = (0xFF << 0) | (0x00 << 8);
    unsafe { core::arch::asm!("msr mair_el1, {0}", "isb", in(reg) mair) }
}

// translation control register: handles addresses
fn setup_tcr() {
    unsafe {
        let tcr: u64 = (16 << 0)  |  // T0SZ: 48-bit VA for TTBR0 (64-48=16)
            (16 << 16) |  // T1SZ: 48-bit VA for TTBR1
            (0b00 << 14)| // TG0: 4K granule
            (0b10 << 30)| // TG1: 4K granule
            (0b11 << 12)| // SH0: inner shareable
            (0b11 << 28)| // SH1: inner shareable
            (0b01 << 10)| // ORGN0: normal WB RAWA
            (0b01 << 26)| // ORGN1
            (0b01 << 8) | // IRGN0
            (0b01 << 24) | // IRGN1
            (0b101 << 32); // IPS: 48-bit PA (256GB)

        core::arch::asm!("msr tcr_el1, {0}", "isb", in(reg) tcr);
    }
}

pub fn initialize_paging_aarch64<'a>(
    hhdm_response: &HhdmResponse,
    memory_map_response: &'a MemoryMapResponse,
    executable_address_response: &ExecutableAddressResponse,
) -> AArchPageTable {
    let mut frame_allocator = FRAME_ALLOCATOR_AARCH64.lock();
    frame_allocator.initialize(memory_map_response.entries());
    drop(frame_allocator);

    let page_table = AArchPageTable::new();

    unsafe extern "C" {
        static __text_end: u8;
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    let k_phys_base = executable_address_response.physical_base();
    let k_virt_base = executable_address_response.virtual_base();

    let text_end = unsafe { &__text_end as *const u8 as u64 };
    let k_start_virt = unsafe { (&__kernel_start as *const u8) as u64 };
    let k_end_virt = unsafe { (&__kernel_end as *const u8) as u64 };

    let k_phys = k_phys_base + (k_start_virt - k_virt_base);

    // map the kernel so we dont crash lol
    page_table.map_range(
        k_start_virt,
        k_phys,
        text_end - k_start_virt,
        kernel_code_flags(),
    );

    page_table.map_range(
        text_end,
        k_phys + (text_end - k_start_virt),
        k_end_virt - text_end,
        kernel_data_flags(),
    );

    // i need to map the entire memory range
    for entry in memory_map_response.entries() {
        if entry.entry_type == EntryType::BAD_MEMORY || entry.entry_type == EntryType::RESERVED {
            continue;
        }

        let flags = if entry.entry_type == EntryType::FRAMEBUFFER {
            device_flags()
        } else {
            kernel_data_flags()
        };

        page_table.map_range(
            entry.base + hhdm_response.offset(),
            entry.base,
            entry.length,
            flags,
        );
    }

    page_table.map_range(0xFFFF_0000_0900_0000, 0x0900_0000, 0x1000, device_flags()); // the UART
    page_table.map_range(0xFFFF_0000_0800_0000, 0x0800_0000, 0x10000, device_flags()); // the GICD
    page_table.map_range(0xFFFF_0000_0801_0000, 0x0801_0000, 0x10000, device_flags()); // the GICC
    page_table.map_page(0xFFFF_0000_0905_0000, 0x0905_0000, device_flags()); // KMI0 (keyboard)
    page_table.map_page(0xFFFF_0000_0906_0000, 0x0906_0000, device_flags()); // KMI1 (mouse)
    setup_mair();
    setup_tcr();

    let stack_phys = KERNEL_STACK.0.as_ptr() as u64 - k_virt_base + k_phys_base;
    let stack_virt = KERNEL_STACK.0.as_ptr() as u64;

    page_table.map_range(stack_virt, stack_phys, 64 * 1024, kernel_data_flags());

    set_page_table(page_table.root_phys);
    tlb_flush();

    return page_table;
}

pub struct AArchPageTable {
    pub root_phys: u64,
}

pub fn alloc_frame() -> Option<u64> {
    let mut frame_allocator = safe_lock(|| FRAME_ALLOCATOR_AARCH64.lock());
    let frame_opt = frame_allocator.allocate_frame();
    drop(frame_allocator);
    return frame_opt;
}

impl AArchPageTable {
    pub fn new() -> AArchPageTable {
        let mut frame_allocator = safe_lock(|| FRAME_ALLOCATOR_AARCH64.lock());
        let root_phys = frame_allocator
            .allocate_frame()
            .expect("Could not allocate frame for page table");

        unsafe {
            core::ptr::write_bytes(phys_to_virt(root_phys), 0, 4096);
        }

        AArchPageTable { root_phys }
    }

    pub fn table_ptr(&self, table_phys: u64, index: usize) -> *mut u64 {
        let table_virt = phys_to_virt(table_phys);

        unsafe { table_virt.add(index) }
    }

    pub fn get_or_create_table(&self, parent_phys: u64, index: usize) -> u64 {
        let entry_ptr = self.table_ptr(parent_phys, index);
        let entry = unsafe { entry_ptr.read_volatile() };

        if entry & VALID != 0 {
            return entry & 0x0000_FFFF_FFFF_F000;
        } else {
            let child = alloc_frame().expect("Could not allocate frame when creating page table");
            unsafe {
                core::ptr::write_bytes(phys_to_virt(child), 0, 4096);
                entry_ptr.write_volatile(child | VALID | TABLE);
            }
            return child;
        }
    }

    pub fn map_page(&self, virt: u64, phys: u64, flags: u64) {
        let l0 = ((virt >> 39) & 0x1FF) as usize;
        let l1 = ((virt >> 30) & 0x1FF) as usize;
        let l2 = ((virt >> 21) & 0x1FF) as usize;
        let l3 = ((virt >> 12) & 0x1FF) as usize;

        let l1_phys = self.get_or_create_table(self.root_phys, l0);
        let l2_phys = self.get_or_create_table(l1_phys, l1);
        let l3_phys = self.get_or_create_table(l2_phys, l2);
        let entry_ptr = self.table_ptr(l3_phys, l3);

        unsafe {
            entry_ptr.write_volatile(phys | flags);
        }
    }

    pub fn map_range(&self, virt: u64, phys: u64, size: u64, flags: u64) {
        let pages = (size + 4095) / 4096;

        for i in 0..pages {
            self.map_page(virt + i * 4096, phys + i * 4096, flags);
        }
    }
}

impl XunilFrameAllocator {
    pub fn allocate_frame(&mut self) -> Option<u64> {
        let hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        while self.region_index < self.usable_region_count {
            let region = self.usable_regions[self.region_index];
            let frame_count = region.length / 4096;

            if self.region_offset < frame_count as usize {
                let addr = region.base + (self.region_offset as u64 * 4096);
                self.region_offset += 1;

                unsafe {
                    core::ptr::write_bytes((addr + hhdm_offset) as *mut u8, 0, 4096);
                }

                return Some(addr);
            }

            self.region_index += 1;
            self.region_offset = 0;
        }

        None
    }
}

pub fn create_and_map_multiple_pages(
    mapper: &mut AArchPageTable,
    page_count: u64,
    base: u64,
    flags: u64,
) {
    let mut frame_allocator = FRAME_ALLOCATOR_AARCH64.lock();

    for i in 0..page_count {
        let frame = frame_allocator.allocate_frame().unwrap();

        let virt = base + i as u64 * 4096;

        mapper.map_page(virt, frame, flags);
    }

    tlb_flush();

    drop(frame_allocator);
}
