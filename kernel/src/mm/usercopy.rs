use alloc::{
    string::{String, ToString},
    vec::Vec,
};
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

pub fn copy_from_user(
    mapper: &mut OffsetPageTable,
    buf: *mut u8,
    src: *const u8,
    len: usize,
) -> Result<(), isize> {
    if len == 0 {
        return Ok(());
    }
    if buf.is_null() || src.is_null() {
        return Err(-14);
    }

    let start = src as u64;
    let end = start
        .checked_add(len as u64)
        .ok_or(-1)
        .map_err(|err| err as isize)?;

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
            if !flags.contains(PageTableFlags::USER_ACCESSIBLE) {
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

pub fn copy_cstr_from_user(
    mapper: &mut OffsetPageTable,
    user_ptr: *const u8,
    max_len: usize,
) -> Result<String, isize> {
    if user_ptr.is_null() {
        return Err(-14);
    }

    let mut buf: Vec<u8> = Vec::with_capacity(64);

    for i in 0..max_len {
        let mut byte = 0u8;
        copy_from_user(mapper, &mut byte as *mut u8, unsafe { user_ptr.add(i) }, 1)?;
        if byte == 0 {
            return core::str::from_utf8(&buf)
                .map(|s| s.to_string())
                .map_err(|_| -84);
        }
        buf.push(byte);
    }

    Err(-36)
}
