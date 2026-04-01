use core::ptr::null;

use x86_64::structures::paging::OffsetPageTable;

use crate::{
    arch::x86_64::paging::XunilFrameAllocator,
    driver::{
        elf::{
            header::{
                ET_DYN, ET_EXEC, ET_REL, Elf64Ehdr, Elf64Rel, Elf64Shdr, SHF_ALLOC, SHT_NOBITS,
                SHT_REL,
            },
            program::load_program,
            reloc::elf_do_reloc,
            section::elf_sheader,
            validation::validate_elf,
        },
        syscall::{malloc, memset},
    },
};

pub fn load_file(
    frame_allocator: &mut XunilFrameAllocator,
    mapper: &mut OffsetPageTable,
    elf_bytes: &[u8],
) -> *const u8 {
    // elf header size
    if elf_bytes.len() < 64 {
        return null();
    }

    let elf_header: &Elf64Ehdr = unsafe { &*(elf_bytes.as_ptr() as *const Elf64Ehdr) };

    if !validate_elf(elf_header, elf_bytes.len()) {
        return null();
    }

    return match elf_header.e_type {
        ET_EXEC => unsafe { load_program(frame_allocator, mapper, elf_header, elf_bytes) },
        ET_DYN => return null(), // TODO
        ET_REL => return null(),
        _ => return null(),
    };
}

// TODO: make ET_REL work
pub unsafe fn elf_load_stage1(hdr: *const Elf64Ehdr) {
    let shdr = unsafe { elf_sheader(hdr) } as *mut Elf64Shdr;
    let mut i: u16 = 0;
    let shnum = unsafe { (*hdr).e_shnum };

    while i < shnum {
        let section: &mut Elf64Shdr = unsafe { &mut *(shdr.add(i as usize)) };

        if section.sh_type == SHT_NOBITS {
            if section.sh_size == 0 {
                continue;
            }

            if (section.sh_flags & SHF_ALLOC) != 1 {
                let mem =
                    unsafe { malloc(section.sh_size as usize, section.sh_addralign as usize) };

                if mem.is_null() {
                    continue; // handle OOM
                }

                unsafe {
                    // zero the memory
                    memset(mem, 0, section.sh_size as usize);
                }
                section.sh_offset = (mem.addr() + hdr.addr()) as u64;
            }
        }

        i += 1;
    }
}

pub unsafe fn elf_load_stage2(hdr: *const Elf64Ehdr) -> i8 {
    let shdr = unsafe { elf_sheader(hdr) } as *mut Elf64Shdr;
    let mut i: u16 = 0;
    let mut idx: u64;
    let shnum = unsafe { (*hdr).e_shnum };

    while i < shnum {
        let section: &mut Elf64Shdr = unsafe { &mut *(shdr.add(i as usize)) };

        if section.sh_type == SHT_REL {
            idx = 0;
            while idx < section.sh_size / section.sh_entsize {
                let reltab: *const Elf64Rel = unsafe {
                    ((hdr as *const u8).add(section.sh_offset as usize) as *const Elf64Rel)
                        .add(idx as usize)
                };

                let result: i8 = unsafe { elf_do_reloc(hdr, reltab, section) };

                if result == -1 {
                    return -1;
                }

                idx += 1;
            }
        }

        i += 1;
    }

    return 0;
}
