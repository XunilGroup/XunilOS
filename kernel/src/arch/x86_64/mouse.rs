// Comment to self: WHY IS THIS SOO HARD

use crate::util::get_bit;
use core::sync::atomic::{AtomicU8, Ordering};

use x86_64::instructions::{interrupts::without_interrupts, port::Port};

static CURRENTLY_RECEIVING_STATE: AtomicU8 = AtomicU8::new(0);
static FLAGS_BYTE: AtomicU8 = AtomicU8::new(0);
static X_DELTA_BYTE: AtomicU8 = AtomicU8::new(0);
static Y_DELTA_BYTE: AtomicU8 = AtomicU8::new(0);

fn wait_input_buffer_clear(command_port: &mut Port<u8>) {
    unsafe {
        loop {
            let status = command_port.read();
            // IBF is bit 1: clear means we can write
            if (status & 0b10) == 0 {
                break;
            }
        }
    }
}

fn wait_output_buffer_full(command_port: &mut Port<u8>) {
    unsafe {
        loop {
            let status = command_port.read();
            // OBF is bit 0: 1 = data available to read
            if (status & 0b1) != 0 {
                break;
            }
        }
    }
}

fn read_ccb(command_port: &mut Port<u8>, data_port: &mut Port<u8>) -> u8 {
    unsafe {
        wait_input_buffer_clear(command_port);
        command_port.write(0x20);
        wait_output_buffer_full(command_port);
        return data_port.read();
    }
}
fn write_ccb(command_port: &mut Port<u8>, data_port: &mut Port<u8>, value: u8) {
    unsafe {
        wait_input_buffer_clear(command_port);
        command_port.write(0x60);
        wait_input_buffer_clear(command_port);
        data_port.write(value);
    }
}

fn check_clear_and_write(command_port: &mut Port<u8>, inst: u8) {
    unsafe {
        wait_input_buffer_clear(command_port);
        command_port.write(inst);
    }
}

fn write_and_expect_output(
    command_port: &mut Port<u8>,
    data_port: &mut Port<u8>,
    inst: u8,
    expected_output: u8,
) -> bool {
    unsafe {
        check_clear_and_write(command_port, inst);
        wait_output_buffer_full(command_port);
        return data_port.read() == expected_output;
    }
}

fn clear_and_expect_output(
    command_port: &mut Port<u8>,
    data_port: &mut Port<u8>,
    expected_output: u8,
) -> bool {
    unsafe {
        wait_input_buffer_clear(command_port);
        wait_output_buffer_full(command_port);
        return data_port.read() == expected_output;
    }
}

pub fn setup_mouse() -> u8 {
    without_interrupts(|| {
        let mut command_port: Port<u8> = Port::new(0x64);
        let mut data_port: Port<u8> = Port::new(0x60);

        unsafe {
            check_clear_and_write(&mut command_port, 0xAD); // disable port 1
            check_clear_and_write(&mut command_port, 0xA7); // disable port 2

            let mut ccb = read_ccb(&mut command_port, &mut data_port);

            ccb = ccb | 0b00000001; // enable keyboard IRQ
            ccb = ccb | 0b00000010; // enable mouse IRQ
            ccb = ccb & 0b11011111; // disable mouse gating
            ccb = ccb & 0b10111111; // disable scancode translation

            write_ccb(&mut command_port, &mut data_port, ccb);

            check_clear_and_write(&mut command_port, 0xAE); // enable port 1
            check_clear_and_write(&mut command_port, 0xA8); // enable port 2

            if !write_and_expect_output(&mut command_port, &mut data_port, 0xA9, 0x00) {
                // mouse test reply doesnt work!
                return 1;
            }

            // Reset Mouse
            wait_input_buffer_clear(&mut command_port);
            command_port.write(0xD4);
            wait_input_buffer_clear(&mut command_port);
            data_port.write(0xFF);

            if !clear_and_expect_output(&mut command_port, &mut data_port, 0xFA) {
                // ACK
                return 2;
            }
            if !clear_and_expect_output(&mut command_port, &mut data_port, 0xAA) {
                // Self-test passed
                return 3;
            }
            if !clear_and_expect_output(&mut command_port, &mut data_port, 0x00) {
                // Mouse ID
                return 4;
            }

            // Enable data reporting
            wait_input_buffer_clear(&mut command_port);
            command_port.write(0xD4);
            wait_input_buffer_clear(&mut command_port);
            data_port.write(0xF4);

            if !clear_and_expect_output(&mut command_port, &mut data_port, 0xFA) {
                return 5; // ACK
            }

            return 6;
        }
    })
}

fn reset_state() {
    CURRENTLY_RECEIVING_STATE.store(0, Ordering::Relaxed);
    FLAGS_BYTE.store(0, Ordering::Relaxed);
    X_DELTA_BYTE.store(0, Ordering::Relaxed);
    Y_DELTA_BYTE.store(0, Ordering::Relaxed);
}

pub fn mouse_interrupt() -> Option<(u8, u8, u8, i16, i16)> {
    let mut command_port: Port<u8> = Port::new(0x64);
    let mut data_port: Port<u8> = Port::new(0x60);
    unsafe {
        if (command_port.read() & 0x20) == 0 {
            // if this interrupt is not for the mouse, return
            return None;
        }

        let byte = data_port.read();

        let state_idx = CURRENTLY_RECEIVING_STATE.fetch_add(1, Ordering::Relaxed);

        if state_idx == 0 {
            if (byte & 0x08) == 0 {
                // if sync bit unset, return
                reset_state();
                return None;
            }

            if (byte & 0b0100_0000) != 0 {
                // if x overflow set, return
                reset_state();
                return None;
            }

            if (byte & 0b1000_0000) != 0 {
                // if y overflow set, return
                reset_state();
                return None;
            }

            FLAGS_BYTE.store(byte, Ordering::Relaxed);
            None
        } else if state_idx == 1 {
            X_DELTA_BYTE.store(byte, Ordering::Relaxed);
            None
        } else if state_idx == 2 {
            Y_DELTA_BYTE.store(byte, Ordering::Relaxed);
            let flags = FLAGS_BYTE.load(Ordering::Relaxed);
            let left_button_pressed = get_bit(flags, 0);
            let right_button_pressed = get_bit(flags, 1);
            let middle_button_pressed = get_bit(flags, 2);
            let x_delta_sign = get_bit(flags, 4);
            let y_delta_sign = get_bit(flags, 5);

            let x_delta: i16 = {
                let x_delta = X_DELTA_BYTE.load(Ordering::Relaxed);
                if x_delta_sign == 1 {
                    (x_delta as i16) - 256
                } else {
                    x_delta as i16
                }
            };

            let y_delta: i16 = -{
                let y_delta = Y_DELTA_BYTE.load(Ordering::Relaxed);
                if y_delta_sign == 1 {
                    (y_delta as i16) - 256
                } else {
                    y_delta as i16
                }
            };

            reset_state();

            Some((
                left_button_pressed,
                right_button_pressed,
                middle_button_pressed,
                x_delta,
                y_delta,
            ))
        } else {
            None
        }
    }
}
