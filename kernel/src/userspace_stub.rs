use alloc::string::ToString;
use x86_64::structures::paging::OffsetPageTable;

use crate::{
    arch::{arch::run_elf, x86_64::paging::XunilFrameAllocator},
    driver::{
        elf::loader::load_file,
        graphics::{
            base::rgb,
            framebuffer::with_framebuffer,
            primitives::{
                circle_filled, circle_outline, rectangle_filled, rectangle_outline,
                triangle_outline,
            },
        },
        mouse::MOUSE,
        serial::with_serial_console,
        timer::TIMER,
    },
    print, println,
    util::test_performance,
};

static CURSOR_BYTES: &[u8] = include_bytes!("../../assets/cursors/default.bmp");
static TEST_ELF_BYTES: &[u8] = include_bytes!("../../assets/helloworld.elf");
const BMP_HEADER_SIZE: usize = 138;
pub const CURSOR_W: usize = 24;
pub const CURSOR_H: usize = 24;

fn unix_to_hms(timestamp: u64) -> (u64, u64, u64) {
    let seconds = timestamp % 86400;
    let h = seconds / 3600;
    let m = (seconds % 3600) / 60;
    let s = seconds % 60;
    (h, m, s)
}

fn boot_animation() {
    let mut i = 1;

    while i < 10 {
        let mut width = 0;
        let mut height = 0;

        with_framebuffer(|fb| {
            fb.clear(rgb(253, 129, 0));
            width = fb.width;
            height = fb.height;
        });

        let text_width = ("XunilOS Loading".len() + ".".repeat(i).len()) * 4 * 2;

        with_serial_console(|serial_console| {
            serial_console.clear(width / 2 - text_width / 2, height / 2)
        });

        println!(
            "{}",
            "XunilOS Loading".to_string() + &".".repeat(i).as_str()
        );

        i += 1;

        with_framebuffer(|fb| fb.swap());

        sleep(200);
    }

    with_serial_console(|serial_console| {
        serial_console.clear(0, 0);
    });

    with_framebuffer(|fb| {
        fb.clear(rgb(253, 129, 0));
    });
}

pub fn userspace_init(
    frame_allocator: &mut XunilFrameAllocator,
    mapper: &mut OffsetPageTable,
) -> ! {
    // this is just a stub

    let entry_point = load_file(frame_allocator, mapper, TEST_ELF_BYTES);
    println!("{:?}", entry_point);

    with_framebuffer(|fb| fb.swap());

    run_elf(entry_point, frame_allocator, mapper);

    loop {}

    // boot_animation();

    // let mut mouse_status = 0;
    // let mut width = 0;
    // let mut height = 0;
    // let mut mouse_x = 100;
    // let mut mouse_y = 100;

    // loop {
    //     with_serial_console(|serial_console| serial_console.clear(0, 0));
    //     with_framebuffer(|fb| fb.clear(rgb(253, 129, 0)));
    //     test_performance(|| {
    //         mouse_status = MOUSE.get_status();
    //         with_framebuffer(|mut fb| {
    //             width = fb.width;
    //             height = fb.height;

    //             let (x_delta, y_delta) = MOUSE.take_motion();

    //             if x_delta != 0 {
    //                 mouse_x = (mouse_x as isize + x_delta as isize).max(0) as usize;
    //             }
    //             if y_delta != 0 {
    //                 mouse_y = (mouse_y as isize + y_delta as isize).max(0) as usize;
    //             }
    //             if mouse_x > width {
    //                 mouse_x = width - CURSOR_W;
    //             }
    //             if mouse_y > height {
    //                 mouse_y = height - CURSOR_H;
    //             }

    //             rectangle_filled(&mut fb, 700, 400, 200, 200, rgb(0, 0, 0));
    //             rectangle_outline(&mut fb, 400, 400, 100, 100, rgb(0, 0, 0));
    //             circle_filled(&mut fb, 200, 200, 100, rgb(0, 0, 0));
    //             circle_outline(&mut fb, 400, 200, 100, rgb(0, 0, 0));
    //             triangle_outline(&mut fb, 100, 400, 200, 400, 150, 600, rgb(0, 0, 0));

    //             let pixels = &CURSOR_BYTES[BMP_HEADER_SIZE..]; // remove header

    //             for row in 0..CURSOR_H {
    //                 let src_row = (CURSOR_H - 1 - row) * CURSOR_W * 4;
    //                 for col in 0..CURSOR_W {
    //                     let i = src_row + col * 4; // 4 because rgba

    //                     let b = pixels[i];
    //                     let g = pixels[i + 1];
    //                     let r = pixels[i + 2];
    //                     let a = pixels[i + 3];

    //                     if a < 255 {
    //                         continue;
    //                     }

    //                     let color = rgb(r, g, b);

    //                     fb.put_pixel((mouse_x + col) as usize, (mouse_y + row) as usize, color);
    //                 }
    //             }
    //         });

    //         let (hours, minutes, seconds) =
    //             unix_to_hms(TIMER.get_date_at_boot() + (TIMER.now().elapsed()) / 1000);

    //         print!(
    //             "{:?}:{:?}:{:?}\nMouse status: {:?}\nDesktop Size: {:?}",
    //             hours,
    //             minutes,
    //             seconds,
    //             mouse_status,
    //             (width, height)
    //         );
    //     });
    //     with_framebuffer(|fb| {
    //         fb.swap();
    //     });
    //     sleep(1000 / 165);
    // }
}
