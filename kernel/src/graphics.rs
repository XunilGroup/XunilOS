use limine::framebuffer::Framebuffer;

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn create_rect(framebuffer: &Framebuffer, x: u64, y: u64, width: u64, height: u64, color: u32) {
    for fb_x in x..x+width {
        for fb_y in y..y+height {
            // Calculate the pixel offset using the framebuffer information we obtained above.
            // We skip `i` scanlines (pitch is provided in bytes) and add `i * 4` to skip `i` pixels forward.
            let bytes_per_pixel = framebuffer.bpp() as u64 / 8;
            let pixel_offset = fb_y * framebuffer.pitch() + fb_x * bytes_per_pixel;

            // Write 0xFFFFFFFF to the provided pixel offset to fill it white.
            unsafe {
                framebuffer
                    .addr()
                    .add(pixel_offset as usize)
                    .cast::<u32>()
                    .write(color)
            };
        }
    }
}