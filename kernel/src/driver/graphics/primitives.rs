use micromath::F32Ext;
use crate::driver::graphics::framebuffer::Framebuffer;
use crate::driver::graphics::base::{rgb, PI};

pub fn line(framebuffer: &mut Framebuffer, mut x0: i64, mut y0: i64, x1: i64, y1: i64, color: u32) {    
    let mut dx = x1 - x0;
    let mut dy = y1 - y0;

    if dx < 0 {
        dx = -dx;
    }

    if dy < 0 {
        dy = -dy;
    }
    
    let step_x = if x0 < x1 {1} else {-1};
    let step_y = if y0 < y1 {1} else {-1};
    
    let mut error = dx - dy;
    let mut e2: i64;
    
    loop {
        framebuffer.put_pixel(x0 as usize, y0 as usize, color);
        
        if x0 == x1 && y0 == y1 {
            break;
        }

        e2 = 2 * error;
        
        if e2 > -dy {
            error -= dy;
            x0 += step_x;
        }   
        
        if e2 < dx {
            error += dx ;
            y0 += step_y;
        }
            
    }
}

pub fn triangle_outline(framebuffer: &mut Framebuffer, x1: usize, y1: usize, x2: usize, y2: usize, x3: usize, y3: usize, color: u32) {
    line(framebuffer, x1 as i64, y1 as i64, x2 as i64, y2 as i64, color);
    line(framebuffer, x1 as i64, y1 as i64, x3 as i64, y3 as i64, color);
    line(framebuffer, x2 as i64, y2 as i64, x3 as i64, y3 as i64, color);
    framebuffer.swap();
}

pub fn circle_outline(framebuffer: &mut Framebuffer, x: usize, y: usize, radius: f32, color: u32) {
    let mut i: f32 = 0.0;

    loop {
        i += 0.1;

        let x1: f32 = radius * (i * PI / 180.0).cos();
        let y1: f32 = radius * (i * PI / 180.0).sin();
        framebuffer.put_pixel((x as f32 + x1) as usize, (y as f32 + y1) as usize, color);

        if i >= 360.0 {
            break
        }
    }
    framebuffer.swap();
}

pub fn circle_filled(framebuffer: &mut Framebuffer, x0: usize, y0: usize, radius: f32, color: u32) {
    let mut x = radius as i64;
    let mut y: i64 = 0;
    let mut x_change: i64 = 1 - (radius as i64 * 2);
    let mut y_change: i64 = 0;
    let mut radius_error: i64 = 0;

    while x >= y {
        let mut i = x0 as i64 - x;
        while i <= x0 as i64 + x {
            framebuffer.put_pixel(i as usize, (y0 as i64 + y) as usize, color);
            framebuffer.put_pixel(i as usize, (y0 as i64 - y) as usize, color);
            i += 1;
        }
        
        let mut i = x0 as i64 - y;
        while i <= x0 as i64 + y {
            framebuffer.put_pixel(i as usize, (y0 as i64 + x) as usize, color);
            framebuffer.put_pixel(i as usize, (y0 as i64 - x) as usize, color);
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
    framebuffer.swap();
}

pub fn rectangle_filled(framebuffer: &mut Framebuffer, x: usize, y: usize, width: usize, height: usize, color: u32, swap: bool) {
    for fb_x in x..x+width {
        for fb_y in y..y+height {
            framebuffer.put_pixel(fb_x, fb_y, color);
        }
    }
    if swap {
        framebuffer.swap();
    }
}

pub fn rectangle_outline(framebuffer: &mut Framebuffer, x: i64, y: i64, width: i64, height: i64, color: u32) {
    line(framebuffer, x, y, x + width, y, color); // bottomleft -> bottomright
    line(framebuffer, x, y + height, x + width, y + height, color); // topleft -> topright 
    line(framebuffer, x, y, x,  y + height, color); // bottomleft -> topleft
    line(framebuffer, x + width, y, x + width, y + height, color); // bottomright -> topright
    framebuffer.swap();
}