use crate::arch::arch::safe_lock;
use x86_64::instructions::port::Port;

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

pub fn setup_kmi() -> u8 {
    safe_lock(|| {
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

pub unsafe fn read_mouse_control() -> u8 {
    let mut command_port: Port<u8> = Port::new(0x64);
    unsafe { command_port.read() }
}

pub unsafe fn read_mouse_data() -> u8 {
    let mut data: Port<u8> = Port::new(0x60);
    unsafe { data.read() }
}

pub unsafe fn write_mouse_control(byte: u8) {
    let mut command_port: Port<u8> = Port::new(0x64);
    unsafe { command_port.write(byte) }
}

pub unsafe fn write_mouse_data(byte: u8) {
    let mut command_port: Port<u8> = Port::new(0x60);
    unsafe { command_port.write(byte) }
}

pub unsafe fn read_keyboard_data() -> u8 {
    unsafe { read_mouse_data() }
}
pub unsafe fn read_keyboard_control() -> u8 {
    unsafe { read_mouse_control() }
}
