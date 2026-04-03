use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

use crate::{arch::x86_64::paging::XunilFrameAllocator, driver::syscall::memset};

pub struct AddressSpace {
    cr3_frame: PhysFrame<Size4KiB>,
    user_stack_top: VirtAddr,
    pub mapper: OffsetPageTable<'static>,
    heap_base: VirtAddr,
    heap_end: VirtAddr,
}
impl AddressSpace {
    pub fn new(frame_allocator: &mut XunilFrameAllocator) -> Option<AddressSpace> {
        let new_pml4 = frame_allocator.allocate_frame()?;

        unsafe {
            let new_pml4_ptr =
                (frame_allocator.hhdm_offset + new_pml4.start_address().as_u64()) as *mut u64;
            core::ptr::write_bytes(new_pml4_ptr, 0, 512);
        }

        let (cur_pml4, pml4_flags) = Cr3::read();

        unsafe {
            let cur_pml4_ptr =
                physical_to_virt_pointer(cur_pml4.start_address(), frame_allocator.hhdm_offset);
            let new_pml4_ptr =
                physical_to_virt_pointer(new_pml4.start_address(), frame_allocator.hhdm_offset);

            for i in 0..512 {
                let val = core::ptr::read(cur_pml4_ptr.add(i));
                core::ptr::write(new_pml4_ptr.add(i), val);
            }
        }

        let mut mapper = unsafe {
            let addr = frame_allocator.hhdm_offset + new_pml4.start_address().as_u64();
            let virtual_addr = VirtAddr::new(addr);
            let level_4_table: *mut PageTable = virtual_addr.as_mut_ptr();
            OffsetPageTable::new(
                &mut *level_4_table,
                VirtAddr::new(frame_allocator.hhdm_offset),
            )
        };

        Some(AddressSpace {
            cr3_frame: new_pml4,
            user_stack_top: VirtAddr::new(0x0000_7fff_0000_0000),
            mapper: mapper,
            heap_base: VirtAddr::new(0x0),
            heap_end: VirtAddr::new(0x0),
        })
    }

    pub fn use_address_space(&mut self) {
        unsafe { Cr3::write(self.cr3_frame, Cr3Flags::empty()) };
    }
}

unsafe fn physical_to_virt_pointer(phys_addr: PhysAddr, hhdm_offset: u64) -> *mut u64 {
    (hhdm_offset + phys_addr.as_u64()) as *mut u64
}
