use crate::driver::elf::header::{Elf64Ehdr, Elf64Shdr, Elf64Sym, SHN_ABS, SHN_UNDEF, STB_WEAK};
use core::ffi::CStr;

// TODO: make ET_REL work

pub unsafe fn elf_sheader(hdr: *const Elf64Ehdr) -> *const Elf64Shdr {
    unsafe { (hdr as *const u8).add((*hdr).e_shoff as usize) as *const Elf64Shdr }
}

pub unsafe fn elf_section(hdr: *const Elf64Ehdr, idx: usize) -> *const Elf64Shdr {
    unsafe { elf_sheader(hdr).add(idx) }
}

// pub unsafe fn elf_str_table(hdr: *const Elf64Ehdr) -> *const u8 {
//     if unsafe { (*hdr).e_shstrndx == SHN_UNDEF } {
//         return core::ptr::null();
//     }

//     let shdr = unsafe { elf_section(hdr, (*hdr).e_shstrndx as usize) };

//     unsafe { (hdr as *const u8).add((*shdr).sh_offset as usize) }
// }

// pub unsafe fn elf_lookup_string(hdr: *const Elf64Ehdr, offset: usize) -> *const u8 {
//     let str_tab: *const u8 = unsafe { elf_str_table(hdr) };

//     if str_tab.is_null() {
//         return core::ptr::null();
//     }

//     return unsafe { str_tab.add(offset) };
// }

pub fn elf_lookup_symbol(name: &CStr) -> *const u8 {
    return core::ptr::null();
}

pub unsafe fn elf_get_symval(hdr: *const Elf64Ehdr, table: u64, idx: u64) -> u64 {
    if table == SHN_UNDEF as u64 || idx == SHN_UNDEF as u64 {
        return 0;
    }

    let symtab: *const Elf64Shdr = unsafe { elf_section(hdr, table as usize) };

    if unsafe { (*symtab).sh_entsize == 0 } {
        return 255; // TODO: replace with error
    }

    let symtab_entries: u64 = unsafe { (*symtab).sh_size / (*symtab).sh_entsize };

    if idx >= symtab_entries {
        return 255; // TODO: replace with error
    }

    let symaddr = unsafe { (hdr as *const u8).add((*symtab).sh_offset as usize) };
    let sym_table = symaddr as *const Elf64Sym;
    let symbol: &Elf64Sym = unsafe { &*sym_table.add(idx as usize) };

    if symbol.st_shndx == SHN_UNDEF {
        // the symbol is external
        let strtab: *const Elf64Shdr = unsafe { elf_section(hdr, (*symtab).sh_link as usize) };
        let name_pointer: *const u8 = unsafe {
            (hdr as *const u8)
                .add((*strtab).sh_offset as usize)
                .add((*symbol).st_name as usize)
        };
        let name = unsafe { CStr::from_ptr(name_pointer as *const core::ffi::c_char) };

        let target = elf_lookup_symbol(name);
        if target == core::ptr::null() {
            if (symbol.st_info >> 4) == STB_WEAK {
                return 0; // Weak symbol initialized as 0
            } else {
                return 255; // // TODO: replace with error Undefined External Symbol
            }
        } else {
            return target as u64;
        }
    } else if symbol.st_shndx == SHN_ABS {
        return symbol.st_value; // absolute symbol easy
    } else {
        let target: *const Elf64Shdr = unsafe { elf_section(hdr, symbol.st_shndx as usize) };
        return unsafe {
            ((hdr as *const u8).add((symbol.st_value + (*target).sh_offset) as usize)) as u64
        };
    }
}
