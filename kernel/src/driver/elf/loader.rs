use core::ptr::null;

use x86_64::structures::paging::OffsetPageTable;

use crate::driver::elf::{
    header::{ET_DYN, ET_EXEC, ET_REL, Elf64Ehdr},
    program::load_program,
    validation::validate_elf,
};

pub fn load_file(mapper: &mut OffsetPageTable, elf_bytes: &[u8]) -> (*const u8, u64) {
    // elf header size
    if elf_bytes.len() < 64 {
        return (null(), 0);
    }

    let elf_header: Elf64Ehdr =
        unsafe { core::ptr::read_unaligned(elf_bytes.as_ptr() as *const Elf64Ehdr) };

    if !validate_elf(&elf_header, elf_bytes.len()) {
        return (null(), 0);
    }

    let elf_header_ptr = elf_bytes.as_ptr() as *const Elf64Ehdr;

    return match elf_header.e_type {
        ET_EXEC => unsafe { load_program(mapper, elf_header_ptr, elf_bytes, false) },
        ET_DYN => unsafe { load_program(mapper, elf_header_ptr, elf_bytes, true) },
        ET_REL => return (null(), 0),
        _ => return (null(), 0),
    };
}
