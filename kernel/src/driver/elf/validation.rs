use crate::driver::elf::header::{
    EI_CLASS, EI_DATA, EI_VERSION, ELF_MAGIC, EM_X86_64, ET_DYN, ET_EXEC, Elf64Ehdr,
};

pub fn validate_elf(elf_header: &Elf64Ehdr, elf_len: usize) -> bool {
    #[allow(unused_mut)]
    let mut required_machine = EM_X86_64;
    #[cfg(target_arch = "aarch64")]
    {
        use crate::driver::elf::header::EM_AARCH64;
        required_machine = EM_AARCH64;
    }

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
