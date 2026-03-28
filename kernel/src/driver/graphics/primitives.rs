use crate::driver::graphics::{base::rgb, font_render::render_text, framebuffer::Framebuffer};
use alloc::{format, string::ToString};
use core::f32::consts::PI;
use micromath::F32Ext;

pub fn line(framebuffer: &mut Framebuffer, x0: usize, y0: usize, x1: usize, y1: usize, color: u32) {
    let mut x0 = x0 as isize;
    let mut y0 = y0 as isize;
    let x1 = x1 as isize;
    let y1 = y1 as isize;

    let mut dx: isize = x1 - x0;
    let mut dy: isize = y1 - y0;

    if dx < 0 {
        dx = -dx;
    }
    if dy < 0 {
        dy = -dy;
    }

    let step_x: isize = if x0 < x1 { 1 } else { -1 };
    let step_y: isize = if y0 < y1 { 1 } else { -1 };

    let mut error: isize = dx - dy;

    loop {
        framebuffer.put_pixel(x0 as usize, y0 as usize, color);

        let e2: isize = 2 * error;
        if e2 > -dy {
            error -= dy;
            x0 += step_x;
        }
        if e2 < dx {
            error += dx;
            y0 += step_y;
        }

        if x0 == x1 && y0 == y1 {
            break;
        }
    }
}

pub fn triangle_outline(
    framebuffer: &mut Framebuffer,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
    x3: usize,
    y3: usize,
    color: u32,
) {
    line(framebuffer, x1, y1, x2, y2, color);
    line(framebuffer, x1, y1, x3, y3, color);
    line(framebuffer, x2, y2, x3, y3, color);
}

pub fn circle_outline(framebuffer: &mut Framebuffer, x: usize, y: usize, radius: f32, color: u32) {
    let mut i: f32 = 0.0;

    loop {
        i += 0.1;

        let x1: f32 = radius * (i * core::f32::consts::PI / 180.0).cos();
        let y1: f32 = radius * (i * PI / 180.0).sin();
        framebuffer.put_pixel((x as f32 + x1) as usize, (y as f32 + y1) as usize, color);

        if i >= 360.0 {
            break;
        }
    }
}

pub fn circle_filled(framebuffer: &mut Framebuffer, x0: usize, y0: usize, radius: f32, color: u32) {
    let mut x = radius as isize;
    let mut y: isize = 0;
    let mut x_change: isize = 1 - (radius as isize * 2);
    let mut y_change: isize = 0;
    let mut radius_error: isize = 0;

    while x >= y {
        let mut i = x0 as isize - x;
        while i <= x0 as isize + x {
            framebuffer.put_pixel(i as usize, (y0 as isize + y) as usize, color);
            framebuffer.put_pixel(i as usize, (y0 as isize - y) as usize, color);
            i += 1;
        }
        let mut i = x0 as isize - y;
        while i <= x0 as isize + y {
            framebuffer.put_pixel(i as usize, (y0 as isize + x) as usize, color);
            framebuffer.put_pixel(i as usize, (y0 as isize - x) as usize, color);
            i += 1;
        }
        y += 1;
        radius_error += y_change;
        y_change += 2;
        if (radius_error * 2) + x_change > 0 {
            x -= 1;
            radius_error += x_change;
            x_change += 2;
        }
    }
}

pub fn rectangle_filled(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    for fb_x in x..x + width {
        for fb_y in y..y + height {
            framebuffer.put_pixel(fb_x, fb_y, color);
        }
    }
}

pub fn rectangle_outline(
    framebuffer: &mut Framebuffer,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
) {
    line(framebuffer, x, y, x + width, y, color); // bottomleft -> bottomright
    line(framebuffer, x, y + height, x + width, y + height, color); // topleft -> topright
    line(framebuffer, x, y, x, y + height, color); // bottomleft -> topleft
    line(framebuffer, x + width, y, x + width, y + height, color); // bottomright -> topright
}
