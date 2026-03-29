use crate::driver::graphics::framebuffer::Framebuffer;

pub fn line(framebuffer: &mut Framebuffer, x0: usize, y0: usize, x1: usize, y1: usize, color: u32) {
    if y0 == y1 {
        let (xa, xb) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        framebuffer.fill_span(xa, y0, xb - xa + 1, color);
        return;
    }
    if x0 == x1 {
        let (ya, yb) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for yy in ya..=yb {
            framebuffer.put_pixel(x0, yy, color);
        }
        return;
    }

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

pub fn circle_outline(
    framebuffer: &mut Framebuffer,
    cx: usize,
    cy: usize,
    radius: usize,
    color: u32,
) {
    let mut x = radius as isize;
    let mut y: isize = 0;
    let mut d = 1 - x;
    let cx = cx as isize;
    let cy = cy as isize;

    #[inline(always)]
    fn plot_points(
        framebuffer: &mut Framebuffer,
        cx: isize,
        cy: isize,
        x: isize,
        y: isize,
        color: u32,
    ) {
        framebuffer.put_pixel((cx + x) as usize, (cy + y) as usize, color);
        framebuffer.put_pixel((cx + y) as usize, (cy + x) as usize, color);
        framebuffer.put_pixel((cx - y) as usize, (cy + x) as usize, color);
        framebuffer.put_pixel((cx - x) as usize, (cy + y) as usize, color);
        framebuffer.put_pixel((cx - x) as usize, (cy - y) as usize, color);
        framebuffer.put_pixel((cx - y) as usize, (cy - x) as usize, color);
        framebuffer.put_pixel((cx + y) as usize, (cy - x) as usize, color);
        framebuffer.put_pixel((cx + x) as usize, (cy - y) as usize, color);
    }

    while y <= x {
        plot_points(framebuffer, cx, cy, x, y, color);
        y += 1;

        if d <= 0 {
            d += 2 * y + 1;
        } else {
            x -= 1;
            d += 2 * (y - x) + 1;
        }
    }
}

pub fn circle_filled(
    framebuffer: &mut Framebuffer,
    x0: usize,
    y0: usize,
    radius: usize,
    color: u32,
) {
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
    for yy in y..y + height {
        framebuffer.fill_span(x, yy, width, color);
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
