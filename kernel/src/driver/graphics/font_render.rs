extern crate font8x8;

use crate::driver::graphics::framebuffer::Framebuffer;
use font8x8::legacy::BASIC_LEGACY;

pub fn rectangle_filled(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    for yy in y..y + height {
        framebuffer.fill_span(x, yy, width, color);
    }
}

pub fn render_char(
    framebuffer: &mut Framebuffer,
    start_x: usize,
    start_y: usize,
    char: usize,
    font_size: usize,
    color: u32,
) {
    if let Some(glyph) = BASIC_LEGACY.get(char) {
        for (row, row_bits) in glyph.iter().enumerate() {
            for bit in 0..8 {
                if (row_bits & (1 << bit)) != 0 {
                    rectangle_filled(
                        framebuffer,
                        start_x + bit * font_size,
                        start_y + row * font_size,
                        font_size,
                        font_size,
                        color,
                    );
                }
            }
        }
    }
}

pub fn render_text(
    framebuffer: &mut Framebuffer,
    start_x: usize,
    start_y: usize,
    text: &str,
    font_size: usize,
    color: u32,
    margin_left: usize,
) -> (usize, usize) {
    let mut x = start_x;
    let mut y = start_y;

    for b in text.bytes() {
        if b == b'\n' {
            y += 12 * font_size;
            x = margin_left;
            continue;
        }

        render_char(framebuffer, x, y, b as usize, font_size, color);
        x += 8 * font_size;
    }

    (x, y)
}
