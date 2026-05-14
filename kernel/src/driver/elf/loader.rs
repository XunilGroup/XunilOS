use core::ptr::null;

#[cfg(target_arch = "x86_64")]
use crate::driver::elf::{
    header::{
        EI_CLASS, EI_DATA, EI_VERSION, ELF_MAGIC, EM_X86_64, ET_DYN, ET_EXEC, ET_REL, Elf64Ehdr,
    },
    program::load_program,
};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::OffsetPageTable;

#[cfg(target_arch = "x86_64")]
pub fn validate_elf(elf_header: &Elf64Ehdr, elf_len: usize) -> bool {
    #[cfg(target_arch = "x86_64")]
    let required_machine = EM_X86_64;

    #[cfg(target_arch = "aarch64")]
    use crate::driver::elf::header::EM_AARCH64;
    #[cfg(target_arch = "aarch64")]
    let required_machine = EM_AARCH64;

    elf_header.e_ident[0..4] == ELF_MAGIC
    // 64 bit
    && elf_header.e_ident[EI_CLASS] == 2
    // little-endian
    && elf_header.e_ident[EI_DATA] == 1
    && elf_header.e_ident[EI_VERSION] == 1
    && elf_header.e_version == 1
    // check architecture
    && elf_header.e_machine == required_machine
    // disallow object files
    && [ET_DYN, ET_EXEC].contains(&elf_header.e_type)
    // standard elf64
    && elf_header.e_phentsize == 56
    && elf_header.e_phnum != 0 // zero program headers
    && (elf_header.e_phoff + (elf_header.e_phnum*elf_header.e_phentsize) as u64) <= elf_len as u64
}

#[cfg(target_arch = "x86_64")]
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
