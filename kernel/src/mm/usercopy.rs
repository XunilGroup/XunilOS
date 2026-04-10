use x86_64::{
    VirtAddr,
    structures::paging::{OffsetPageTable, PageTableFlags, Translate, mapper::TranslateResult},
};

pub fn copy_to_user(
    mapper: &mut OffsetPageTable,
    buf: *mut u8,
    src: *const u8,
    len: usize,
) -> Result<(), isize> {
    let start = buf as u64;
    let end = start + len as u64;
    let mut page_addr = start & !0xFFF;

    while page_addr < end {
        let translate_result = mapper.translate(VirtAddr::new(page_addr));
        #[allow(non_shorthand_field_patterns)]
        if let TranslateResult::Mapped {
            frame: _,
            offset: _,
            flags: flags,
        } = translate_result
        {
            if flags.contains(PageTableFlags::USER_ACCESSIBLE)
                && flags.contains(PageTableFlags::WRITABLE)
            {
            } else {
                return Err(-13);
            }
        } else {
            return Err(-1);
        }
        page_addr += 0x1000;
    }

    unsafe { core::ptr::copy_nonoverlapping(src, buf, len) };
    Ok(())
}
