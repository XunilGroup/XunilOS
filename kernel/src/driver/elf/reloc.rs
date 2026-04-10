use crate::driver::elf::{
    header::{Elf64Ehdr, Elf64Rel, Elf64Shdr, R_X86_64_64},
    section::{elf_get_symval, elf_section},
};

pub unsafe fn elf_do_reloc(
    hdr: *const Elf64Ehdr,
    rel: *const Elf64Rel,
    reltab: *mut Elf64Shdr,
) -> i8 {
    let target = unsafe { elf_section(hdr, (*reltab).sh_info as usize) };
    let addr = unsafe { (hdr as *const u8).add((*target).sh_offset as usize) };
    let reference = unsafe { addr.add((*rel).r_offset as usize) as *mut u64 };

    let symval;

    if unsafe { ((*rel).r_info) >> 32 } != 0 {
        symval = unsafe { elf_get_symval(hdr, (*reltab).sh_link as u64, (*rel).r_info >> 32) };
        if symval == 1 {
            return -1;
        }

        match unsafe { ((*rel).r_info & 0xffff_ffff) as u32 } {
            x if x == R_X86_64_64 as u32 => unsafe { *reference = symval.wrapping_add(*reference) },
            _ => return -1,
        }

        return symval as i8;
    }

    return 0;
}
