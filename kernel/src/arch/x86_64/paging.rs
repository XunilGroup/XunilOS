use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

use limine::memory_map::{Entry, EntryType};

unsafe fn active_level_4_table(mem_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table, _) = Cr3::read();

    let physical_addr = level_4_table.start_address();
    let virtual_addr = mem_offset + physical_addr.as_u64();
    let page_table_ptr: *mut PageTable = virtual_addr.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub unsafe fn initialize_paging(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    unsafe {
        let level_4_table = active_level_4_table(physical_memory_offset);
        OffsetPageTable::new(level_4_table, physical_memory_offset)
    }
}

pub struct XunilFrameAllocator<'a> {
    memory_map: &'a [&'a Entry],
    region_index: usize,
    region_offset: usize,
}

impl<'a> XunilFrameAllocator<'a> {
    pub fn new(memory_map: &'a [&'a Entry]) -> Self {
        let region_index = memory_map
            .iter()
            .position(|region| region.entry_type == EntryType::USABLE)
            .unwrap();

        Self {
            memory_map,
            region_index,
            region_offset: 0,
        }
    }
}

unsafe impl<'a> FrameAllocator<Size4KiB> for XunilFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        loop {
            let region = self
                .memory_map
                .iter()
                .filter(|region| region.entry_type == EntryType::USABLE)
                .nth(self.region_index)?;

            let frame_count = region.length / 4096;

            if self.region_offset < frame_count as usize {
                let addr = region.base + (self.region_offset as u64 * 4096);
                self.region_offset += 1;
                return Some(PhysFrame::containing_address(PhysAddr::new(addr)));
            }

            self.region_index += 1;
            self.region_offset = 0;
        }
    }
}
