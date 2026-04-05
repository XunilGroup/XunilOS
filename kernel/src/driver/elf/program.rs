use core::{
    ffi::CStr,
    ptr::{null, null_mut},
};

use alloc::vec::Vec;
use x86_64::{
    VirtAddr,
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, Size4KiB, mapper::MapToError,
    },
};

use crate::{
    arch::arch::FRAME_ALLOCATOR,
    driver::{
        elf::{
            header::{
                DT_JMPREL, DT_NEEDED, DT_NULL, DT_PLTREL, DT_PLTRELSZ, DT_RELA, DT_RELAENT,
                DT_RELASZ, DT_STRSZ, DT_STRTAB, DT_SYMENT, DT_SYMTAB, Elf64Dyn, Elf64Ehdr,
                Elf64Phdr, Elf64Rela, Elf64Sym, PF_X, PT_DYNAMIC, PT_LOAD, R_X86_64_64,
                R_X86_64_GLOB_DAT, R_X86_64_JUMP_SLOT, R_X86_64_NONE, R_X86_64_RELATIVE, SHN_UNDEF,
                STB_WEAK,
            },
            reloc::elf_do_reloc,
        },
        syscall::memset,
    },
    util::{align_down, align_up},
};

const PAGE_SIZE: u64 = 4096;

pub unsafe fn elf_pheader(hdr: *const Elf64Ehdr) -> *const Elf64Phdr {
    unsafe { (hdr as *const u8).add((*hdr).e_phoff as usize) as *const Elf64Phdr }
}

pub fn get_vaddr(phdr: *const Elf64Phdr, load_bias: u64) -> *mut u8 {
    unsafe { ((*phdr).p_vaddr + load_bias) as *mut u8 }
}

pub unsafe fn load_program(
    mapper: &mut OffsetPageTable,
    hdr: *const Elf64Ehdr,
    elf_bytes: &[u8],
    pie: bool,
) -> (*const u8, u64) {
    let phdr = unsafe { elf_pheader(hdr) };
    let phnum = unsafe { (*hdr).e_phnum };
    let mut program_headers: Vec<*const Elf64Phdr> = Vec::new();
    let mut pt_dynamic_header: *const Elf64Phdr = core::ptr::null();

    for i in 0..phnum {
        let program_header = unsafe { phdr.add(i as usize) };
        let p_type = unsafe { (*program_header).p_type };

        if p_type == PT_LOAD {
            program_headers.push(program_header);
        } else if p_type == PT_DYNAMIC {
            pt_dynamic_header = program_header as *const Elf64Phdr;
        }
    }

    if !pie {
        for program_header in program_headers {
            load_segment_to_memory(mapper, program_header, elf_bytes, 0);
        }

        let mut highest_seg = 0;

        for i in 0..phnum {
            let program_header = unsafe { phdr.add(i as usize) };
            let seg_end = unsafe { (*program_header).p_vaddr + (*program_header).p_memsz };
            if seg_end > highest_seg {
                highest_seg = seg_end;
            }
        }

        return (
            unsafe { (*hdr).e_entry as *const u8 },
            align_up(highest_seg as u64, 4096),
        );
    } else {
        let base_address = 0x0000_0100_0000; // TODO: add per-process memory
        let min_vaddr = align_down(
            program_headers
                .iter()
                .map(|&phdr| get_vaddr(phdr, 0) as u64)
                .min()
                .unwrap(),
            4096,
        );

        let load_bias = base_address - min_vaddr;

        let mut highest_seg = 0;

        for i in 0..phnum {
            let program_header = unsafe { phdr.add(i as usize) };
            let seg_end =
                unsafe { (*program_header).p_vaddr + (*program_header).p_memsz + load_bias };
            if seg_end > highest_seg {
                highest_seg = seg_end;
            }
        }

        for program_header in program_headers {
            load_segment_to_memory(mapper, program_header, elf_bytes, load_bias);
        }

        if pt_dynamic_header.is_null() {
            return (null(), 0);
        }

        parse_dyn(
            hdr,
            pt_dynamic_header,
            unsafe { elf_bytes.as_ptr().add((*phdr).p_offset as usize) as *const Elf64Dyn },
            load_bias,
        );

        return (
            unsafe { ((*hdr).e_entry + load_bias) as *const u8 },
            align_up(highest_seg as u64, 4096),
        );
    }
}

fn cstr_from_strtab(
    strtab_ptr: *const u8,
    strtab_size: u64,
    off: u32,
) -> Option<&'static core::ffi::CStr> {
    let off = off as u64;
    if strtab_ptr.is_null() || off >= strtab_size {
        return None;
    }

    let mut i = off;

    while i < strtab_size {
        let b = unsafe { *strtab_ptr.add(i as usize) };
        if b == 0 {
            let start = unsafe { strtab_ptr.add(off as usize) } as *const core::ffi::c_char;
            return Some(unsafe { CStr::from_ptr(start as *const i8) });
        }
        i += 1;
    }

    None
}

pub fn dyn_get_symaddr(
    strtab_ptr: *const u8,
    strtab_size: u64,
    symtab_ptr: *const Elf64Sym,
    idx: u64,
    load_bias: u64,
) -> Result<u64, ()> {
    let sym = unsafe { &*symtab_ptr.add(idx as usize) };
    if sym.st_shndx != SHN_UNDEF {
        return Ok(load_bias + sym.st_value);
    }

    let name = cstr_from_strtab(strtab_ptr, strtab_size, idx as u32);

    let bind = sym.st_info >> 4;
    if bind == STB_WEAK { Ok(0) } else { Err(()) }
}

fn apply_relocations(
    hdr: *const Elf64Ehdr,
    rela_ptr: *mut Elf64Rela,
    rela_table_size: u64,
    symtab_ptr: *const Elf64Sym,
    strtab_ptr: *const u8,
    strtab_size: u64,
    load_bias: u64,
) {
    for i in 0..rela_table_size as usize / size_of::<Elf64Rela>() {
        let rela_ptr = unsafe { rela_ptr.add(i) };
        let ptr = unsafe { (load_bias + (*rela_ptr).r_offset) as *mut u64 };
        let mut value: u64 = 0;
        match unsafe { ((*rela_ptr).r_info & 0xffff_ffff) as u32 } {
            x if x == R_X86_64_RELATIVE as u32 => unsafe {
                value = (load_bias as i64 + (*rela_ptr).r_addend) as u64;
            },
            x if x == R_X86_64_64 as u32 => unsafe {
                value = (dyn_get_symaddr(
                    strtab_ptr,
                    strtab_size,
                    symtab_ptr,
                    (*rela_ptr).r_info >> 32,
                    load_bias,
                )
                .unwrap() as i64
                    + (*rela_ptr).r_addend) as u64;
            },
            x if [R_X86_64_GLOB_DAT, R_X86_64_JUMP_SLOT].contains(&x) => unsafe {
                value = dyn_get_symaddr(
                    strtab_ptr,
                    strtab_size,
                    symtab_ptr,
                    (*rela_ptr).r_info >> 32,
                    load_bias,
                )
                .unwrap() as u64;
            },
            x if x == R_X86_64_NONE as u32 => {
                continue; // explicitly do nothing
            }
            _ => {
                continue;
            }
        }
        unsafe { *ptr = value };
    }
}

fn parse_dyn(
    hdr: *const Elf64Ehdr,
    phdr: *const Elf64Phdr,
    dyn_hdr_original: *const Elf64Dyn,
    load_bias: u64,
) {
    let mut i: usize = 0;
    let file_size: u64 = unsafe { (*phdr).p_filesz };

    let mut rela_ptr: *mut Elf64Rela = null_mut();
    let mut rela_table_size: u64 = 0;

    let mut symtab_ptr: *const Elf64Sym = null();

    let mut strtab_ptr: *const u8 = null();
    let mut strtab_size: u64 = 0;

    let max = file_size as usize / size_of::<Elf64Dyn>();

    while unsafe { (*dyn_hdr_original.add(i)).d_tag != DT_NULL } && i < max {
        let dyn_hdr = unsafe { *(dyn_hdr_original.add(i)) };
        match dyn_hdr.d_tag {
            DT_RELA => rela_ptr = (dyn_hdr.d_val + load_bias) as *mut Elf64Rela,
            DT_RELASZ => {
                rela_table_size = dyn_hdr.d_val;
            }
            DT_JMPREL => {
                // TODO: plt relocations
            }
            DT_PLTREL => {
                // TODO: plt relocations
            }
            DT_PLTRELSZ => {
                // TODO: plt relocations
            }
            DT_NEEDED => {
                // TODO: do dynamic loading
            }
            DT_SYMTAB => symtab_ptr = (dyn_hdr.d_val + load_bias) as *const Elf64Sym,
            DT_STRTAB => strtab_ptr = (dyn_hdr.d_val + load_bias) as *const u8,
            DT_STRSZ => {
                strtab_size = dyn_hdr.d_val;
            }
            _ => (),
        }

        i += 1;
    }

    if rela_ptr.is_null() || symtab_ptr.is_null() || strtab_ptr.is_null() {
        return;
    }

    apply_relocations(
        hdr,
        rela_ptr,
        rela_table_size,
        symtab_ptr,
        strtab_ptr,
        strtab_size,
        load_bias,
    );
}

pub fn load_segment_to_memory(
    mapper: &mut OffsetPageTable,
    phdr: *const Elf64Phdr,
    elf_bytes: &[u8],
    load_bias: u64,
) {
    let mem_size: u64 = unsafe { (*phdr).p_memsz };
    let p_offset: u64 = unsafe { (*phdr).p_offset };
    let file_size: u64 = unsafe { (*phdr).p_filesz };

    let vaddr: *mut u8 = get_vaddr(phdr, load_bias);

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

    let mut frame_allocator = FRAME_ALLOCATOR.lock();

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::<Size4KiB>::FrameAllocationFailed)
            .expect("test");
        unsafe {
            mapper
                .map_to(page, frame, flags, &mut *frame_allocator)
                .map_err(|e| e)
                .expect("test")
                .flush();
        }
    }

    drop(frame_allocator);

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
