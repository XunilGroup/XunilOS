use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB, mapper::MapToError,
    },
};

use crate::{
    arch::x86_64::paging::XunilFrameAllocator,
    driver::{
        elf::header::{Elf64Ehdr, Elf64Phdr, PF_X, PT_LOAD},
        syscall::memset,
    },
    util::{align_down, align_up},
};

const PAGE_SIZE: u64 = 4096;

pub unsafe fn elf_pheader(hdr: *const Elf64Ehdr) -> *const Elf64Phdr {
    unsafe { (hdr as *const u8).add((*hdr).e_phoff as usize) as *const Elf64Phdr }
}

pub unsafe fn load_program(
    frame_allocator: &mut XunilFrameAllocator,
    mapper: &mut OffsetPageTable,
    hdr: *const Elf64Ehdr,
    elf_bytes: &[u8],
) -> *const u8 {
    let phdr = unsafe { elf_pheader(hdr) };
    let phnum = unsafe { (*hdr).e_phnum };

    for i in 0..phnum {
        let program_header = unsafe { phdr.add(i as usize) };

        if unsafe { (*program_header).p_type } == PT_LOAD {
            load_segment_to_memory(frame_allocator, mapper, program_header, elf_bytes);
        }
    }

    return unsafe { (*hdr).e_entry as *const u8 };
}

pub fn load_segment_to_memory(
    frame_allocator: &mut XunilFrameAllocator,
    mapper: &mut OffsetPageTable,
    phdr: *const Elf64Phdr,
    elf_bytes: &[u8],
) {
    let mem_size: u64 = unsafe { (*phdr).p_memsz };
    let p_offset: u64 = unsafe { (*phdr).p_offset };
    let file_size: u64 = unsafe { (*phdr).p_filesz };

    let vaddr: *mut u8 = unsafe { (*phdr).p_vaddr as *mut u8 };

    if p_offset > elf_bytes.len() as u64
        || file_size > elf_bytes.len() as u64
        || p_offset + file_size > elf_bytes.len() as u64
    {
        return;
    } // invalid, could read past it's memory

    if file_size > mem_size {
        return;
    }

    let seg_start = align_down(vaddr as u64, PAGE_SIZE);
    let seg_end = align_up(vaddr as u64 + mem_size, PAGE_SIZE);

    let mut flags =
        PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;

    if unsafe { ((*phdr).p_flags & PF_X) != 0 } {
    } else {
        flags |= PageTableFlags::NO_EXECUTE;
    }

    let start_page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(seg_start));
    let end_page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(seg_end - 1));
    let page_range = Page::range_inclusive(start_page, end_page);

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::<Size4KiB>::FrameAllocationFailed)
            .expect("test");
        unsafe {
            mapper
                .map_to(page, frame, flags, frame_allocator)
                .map_err(|e| e)
                .expect("test")
                .flush();
        }
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            elf_bytes.as_ptr().add(p_offset as usize),
            vaddr,
            file_size as usize,
        );

        if mem_size > file_size {
            memset(
                vaddr.add(file_size as usize),
                0,
                (mem_size - file_size) as usize,
            );
        }
    };
}
