use core::ptr::null;

#[cfg(target_arch = "x86_64")]
use crate::arch::x86_64::paging::create_and_map_multiple_pages;
#[allow(unused_imports)]
use crate::driver::elf::{
    header::{
        EI_CLASS, EI_DATA, EI_VERSION, ELF_MAGIC, EM_AARCH64, EM_X86_64, ET_DYN, ET_EXEC, ET_REL,
        Elf64Ehdr,
    },
    program::load_program,
};

#[cfg(target_arch = "aarch64")]
use crate::arch::aarch64::paging::{
    AArchPageTable, create_and_map_multiple_pages, user_data_flags,
};
use crate::{mm::address_space::AddressSpace, println, task::scheduler::SCHEDULER};
#[cfg(target_arch = "x86_64")]
use x86_64::structures::paging::{OffsetPageTable, PageTableFlags};

#[cfg(target_arch = "aarch64")]
type PageTable = AArchPageTable;

#[cfg(target_arch = "x86_64")]
type PageTable<'a> = OffsetPageTable<'a>;

pub fn validate_elf(elf_header: &Elf64Ehdr, elf_len: usize) -> bool {
    #[cfg(target_arch = "x86_64")]
    let required_machine = EM_X86_64;

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

pub fn load_file(mapper: &mut PageTable, elf_bytes: &[u8]) -> (*const u8, u64) {
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

pub fn run_elf(file_bytes: &[u8], should_swapgs: bool) {
    let stack_base: u64 = 0x0000_7fff_0000_0000;
    let page_count = 4096; // 16 mib
    let page_size = 0x1000u64;
    let stack_top = stack_base + (page_count as u64 * page_size);

    if let Some(mut address_space) = AddressSpace::new() {
        address_space.use_address_space();

        let (entry_point, heap_base) = load_file(&mut address_space.mapper, file_bytes);

        println!("Entry point: {:?}", entry_point);

        let process_pid = SCHEDULER
            .spawn_process(entry_point as u64, stack_top, heap_base)
            .unwrap();

        #[cfg(target_arch = "aarch64")]
        create_and_map_multiple_pages(
            &mut address_space.mapper,
            page_count,
            stack_base,
            user_data_flags(),
        );

        #[cfg(target_arch = "x86_64")]
        create_and_map_multiple_pages(
            &mut address_space.mapper,
            page_count,
            stack_base,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        );

        SCHEDULER.with_process(process_pid, |process| {
            process.address_space = Some(address_space)
        });

        SCHEDULER.switch_to(process_pid, should_swapgs);
    } else {
        return;
    };
}
