#[cfg(target_arch = "x86_64")]
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        FrameAllocator, OffsetPageTable, PageTable as X86PageTable, PhysFrame, Size4KiB,
    },
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::AArchPageTable;

#[cfg(target_arch = "x86_64")]
use crate::arch::arch::{FRAME_ALLOCATOR, HHDM_OFFSET};

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

#[cfg(target_arch = "x86_64")]
pub struct AddressSpace {
    cr3_frame: PhysFrame<Size4KiB>,
    pub mapper: PageTable<'static>,
}

#[cfg(target_arch = "aarch64")]
pub struct AddressSpace {
    ttbr0_phys: u64,
    pub mapper: PageTable,
}

#[cfg(target_arch = "x86_64")]
impl AddressSpace {
    pub fn new() -> Option<AddressSpace> {
        let mut frame_allocator = FRAME_ALLOCATOR.lock();
        let new_pml4 = frame_allocator.allocate_frame()?;

        let hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
        unsafe {
            let new_pml4_ptr = (hhdm_offset + new_pml4.start_address().as_u64()) as *mut u64;
            core::ptr::write_bytes(new_pml4_ptr, 0, 512);
        }

        let (cur_pml4, _) = Cr3::read();

        unsafe {
            let cur_pml4_ptr = physical_to_virt_pointer(cur_pml4.start_address());
            let new_pml4_ptr = physical_to_virt_pointer(new_pml4.start_address());

            for i in 256..512 {
                let val = core::ptr::read(cur_pml4_ptr.add(i));
                core::ptr::write(new_pml4_ptr.add(i), val);
            }
        }

        let mapper = unsafe {
            let addr = hhdm_offset + new_pml4.start_address().as_u64();
            let virtual_addr = VirtAddr::new(addr);
            let level_4_table: *mut X86PageTable = virtual_addr.as_mut_ptr();
            OffsetPageTable::new(&mut *level_4_table, VirtAddr::new(hhdm_offset))
        };

        drop(frame_allocator);

        Some(AddressSpace {
            cr3_frame: new_pml4,
            mapper: mapper,
        })
    }

    pub fn use_address_space(&mut self) {
        unsafe { Cr3::write(self.cr3_frame, Cr3Flags::empty()) };
    }
}

#[cfg(target_arch = "aarch64")]
impl AddressSpace {
    pub fn new() -> Option<AddressSpace> {
        let page_table = AArchPageTable::new();
        let ttbr0_phys = page_table.root_phys;
        Some(AddressSpace {
            ttbr0_phys,
            mapper: page_table,
        })
    }
    pub fn use_address_space(&mut self) {
        unsafe {
            core::arch::asm!(
                "msr ttbr0_el1, {root}",
                "isb",
                "dsb ishst",
                "tlbi vmalle1is",
                "dsb ish",
                "isb",
                root = in(reg) self.mapper.root_phys
            )
        };
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn physical_to_virt_pointer(phys_addr: PhysAddr) -> *mut u64 {
    let hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);
    (hhdm_offset + phys_addr.as_u64()) as *mut u64
}
