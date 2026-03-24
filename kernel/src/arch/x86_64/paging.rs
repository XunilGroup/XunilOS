use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PageTableFlags as Flags,
        PhysFrame, Size2MiB, Size4KiB,
    },
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
    next: usize,
    memory_map: &'a [&'a Entry],
}

impl<'a> XunilFrameAllocator<'a> {
    pub fn new(memory_map: &'a [&'a Entry]) -> Self {
        Self {
            next: 0,
            memory_map,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable = regions.filter(|region| region.entry_type == EntryType::USABLE);
        let ranges = usable
            .map(|usable_region| usable_region.base..usable_region.base + usable_region.length);
        let frame_addresses = ranges.flat_map(|r| r.step_by(4096));

        frame_addresses
            .map(|frame_address| PhysFrame::containing_address(PhysAddr::new(frame_address)))
    }
}

unsafe impl<'a> FrameAllocator<Size4KiB> for XunilFrameAllocator<'a> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub unsafe fn example_mapping(
    page: Page<Size2MiB>,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(0x0000_1234_4000_0000));
    let flags = Flags::PRESENT | Flags::WRITABLE;
    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_to_result.expect("map_to failed").flush();
}
