use spin::mutex::Mutex;
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB},
};

use limine::memory_map::{Entry, EntryType};

use crate::util::align_up;

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

#[derive(Clone, Copy)]
struct UsableRegion {
    base: u64,
    length: u64,
}

const EMPTY_REGION: UsableRegion = UsableRegion { base: 0, length: 0 };

pub struct XunilFrameAllocator {
    pub hhdm_offset: u64,
    usable_regions: [UsableRegion; 1024],
    usable_region_count: usize,
    region_index: usize,
    region_offset: usize,
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

    pub fn initialize(&mut self, hhdm_offset: u64, memory_map: &[&Entry]) {
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

        self.hhdm_offset = hhdm_offset;
        self.usable_regions = regions;
        self.usable_region_count = count;
        self.region_index = 0;
        self.region_offset = 0;
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
                return Some(PhysFrame::containing_address(PhysAddr::new(addr)));
            }

            self.region_index += 1;
            self.region_offset = 0;
        }

        None
    }
}

pub static FRAME_ALLOCATOR_X86_64: Mutex<XunilFrameAllocator> =
    Mutex::new(XunilFrameAllocator::new());
