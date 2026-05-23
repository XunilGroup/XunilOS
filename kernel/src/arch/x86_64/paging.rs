use spin::mutex::Mutex;
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame,
        Size4KiB,
    },
};

use crate::{
    arch::arch::{HHDM_OFFSET, XunilFrameAllocator, serial_print},
    println,
};

unsafe fn active_level_4_table(mem_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table, _) = Cr3::read();

    let physical_addr = level_4_table.start_address();
    let virtual_addr = mem_offset + physical_addr.as_u64();
    let page_table_ptr: *mut PageTable = virtual_addr.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub unsafe fn initialize_paging_x86_64(
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

unsafe impl FrameAllocator<Size4KiB> for XunilFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        while self.region_index < self.usable_region_count {
            let region = self.usable_regions[self.region_index];
            let frame_count = region.length / 4096;

            if self.region_offset < frame_count as usize {
                let addr = region.base + (self.region_offset as u64 * 4096);
                self.region_offset += 1;

                let hhdm_offset = HHDM_OFFSET.load(core::sync::atomic::Ordering::Relaxed);

                unsafe {
                    core::ptr::write_bytes((addr + hhdm_offset) as *mut u8, 0, 4096);
                }

                return Some(PhysFrame::containing_address(PhysAddr::new(addr)));
            }

            self.region_index += 1;
            self.region_offset = 0;
        }

        None
    }
}

pub fn create_and_map_multiple_pages(
    mapper: &mut OffsetPageTable,
    page_count: u64,
    base: u64,
    flags: PageTableFlags,
) {
    let mut frame_allocator = FRAME_ALLOCATOR_X86_64.lock();

    for i in 0..page_count {
        let frame = frame_allocator.allocate_frame().unwrap();

        let virt_addr = VirtAddr::new(base + i as u64 * 4096);
        let page = Page::<Size4KiB>::containing_address(virt_addr);

        unsafe {
            mapper
                .map_to(page, frame, flags, &mut *frame_allocator)
                .unwrap()
                .flush();
        }
    }
    drop(frame_allocator);
}

pub static FRAME_ALLOCATOR_X86_64: Mutex<XunilFrameAllocator> =
    Mutex::new(XunilFrameAllocator::new());
