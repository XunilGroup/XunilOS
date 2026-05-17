pub const KEY_RESERVED: u8 = 0;
pub const KEY_ESC: u8 = 1;
pub const KEY_1: u8 = 2;
pub const KEY_2: u8 = 3;
pub const KEY_3: u8 = 4;
pub const KEY_4: u8 = 5;
pub const KEY_5: u8 = 6;
pub const KEY_6: u8 = 7;
pub const KEY_7: u8 = 8;
pub const KEY_8: u8 = 9;
pub const KEY_9: u8 = 10;
pub const KEY_0: u8 = 11;
pub const KEY_MINUS: u8 = 12;
pub const KEY_EQUAL: u8 = 13;
pub const KEY_BACKSPACE: u8 = 14;
pub const KEY_TAB: u8 = 15;
pub const KEY_Q: u8 = 16;
pub const KEY_W: u8 = 17;
pub const KEY_E: u8 = 18;
pub const KEY_R: u8 = 19;
pub const KEY_T: u8 = 20;
pub const KEY_Y: u8 = 21;
pub const KEY_U: u8 = 22;
pub const KEY_I: u8 = 23;
pub const KEY_O: u8 = 24;
pub const KEY_P: u8 = 25;
pub const KEY_LEFTBRACE: u8 = 26;
pub const KEY_RIGHTBRACE: u8 = 27;
pub const KEY_ENTER: u8 = 28;
pub const KEY_LEFTCTRL: u8 = 29;
pub const KEY_A: u8 = 30;
pub const KEY_S: u8 = 31;
pub const KEY_D: u8 = 32;
pub const KEY_F: u8 = 33;
pub const KEY_G: u8 = 34;
pub const KEY_H: u8 = 35;
pub const KEY_J: u8 = 36;
pub const KEY_K: u8 = 37;
pub const KEY_L: u8 = 38;
pub const KEY_SEMICOLON: u8 = 39;
pub const KEY_APOSTROPHE: u8 = 40;
pub const KEY_GRAVE: u8 = 41;
pub const KEY_LEFTSHIFT: u8 = 42;
pub const KEY_BACKSLASH: u8 = 43;
pub const KEY_Z: u8 = 44;
pub const KEY_X: u8 = 45;
pub const KEY_C: u8 = 46;
pub const KEY_V: u8 = 47;
pub const KEY_B: u8 = 48;
pub const KEY_N: u8 = 49;
pub const KEY_M: u8 = 50;
pub const KEY_COMMA: u8 = 51;
pub const KEY_DOT: u8 = 52;
pub const KEY_SLASH: u8 = 53;
pub const KEY_RIGHTSHIFT: u8 = 54;
pub const KEY_KPASTERISK: u8 = 55;
pub const KEY_LEFTALT: u8 = 56;
pub const KEY_SPACE: u8 = 57;
pub const KEY_CAPSLOCK: u8 = 58;
pub const KEY_F1: u8 = 59;
pub const KEY_F2: u8 = 60;
pub const KEY_F3: u8 = 61;
pub const KEY_F4: u8 = 62;
pub const KEY_F5: u8 = 63;
pub const KEY_F6: u8 = 64;
pub const KEY_F7: u8 = 65;
pub const KEY_F8: u8 = 66;
pub const KEY_F9: u8 = 67;
pub const KEY_F10: u8 = 68;
pub const KEY_NUMLOCK: u8 = 69;
pub const KEY_SCROLLLOCK: u8 = 70;
pub const KEY_KP7: u8 = 71;
pub const KEY_KP8: u8 = 72;
pub const KEY_KP9: u8 = 73;
pub const KEY_KPMINUS: u8 = 74;
pub const KEY_KP4: u8 = 75;
pub const KEY_KP5: u8 = 76;
pub const KEY_KP6: u8 = 77;
pub const KEY_KPPLUS: u8 = 78;
pub const KEY_KP1: u8 = 79;
pub const KEY_KP2: u8 = 80;
pub const KEY_KP3: u8 = 81;
pub const KEY_KP0: u8 = 82;
pub const KEY_KPDOT: u8 = 83;
pub const KEY_RIGHTCTRL: u8 = 97;
pub const KEY_UP: u8 = 103;
pub const KEY_LEFT: u8 = 105;
pub const KEY_RIGHT: u8 = 106;
pub const KEY_DOWN: u8 = 108;

pub const KEY_TO_CHAR: [Option<(char, char)>; 84] = [
    None,                   // 0  KEY_RESERVED
    Some(('\x1b', '\x1b')), // 1  KEY_ESC
    Some(('1', '!')),       // 2  KEY_1
    Some(('2', '@')),       // 3  KEY_2
    Some(('3', '#')),       // 4  KEY_3
    Some(('4', '$')),       // 5  KEY_4
    Some(('5', '%')),       // 6  KEY_5
    Some(('6', '^')),       // 7  KEY_6
    Some(('7', '&')),       // 8  KEY_7
    Some(('8', '*')),       // 9  KEY_8
    Some(('9', '(')),       // 10 KEY_9
    Some(('0', ')')),       // 11 KEY_0
    Some(('-', '_')),       // 12 KEY_MINUS
    Some(('=', '+')),       // 13 KEY_EQUAL
    Some(('\x08', '\x08')), // 14 KEY_BACKSPACE
    Some(('\t', '\t')),     // 15 KEY_TAB
    Some(('q', 'Q')),       // 16 KEY_Q
    Some(('w', 'W')),       // 17 KEY_W
    Some(('e', 'E')),       // 18 KEY_E
    Some(('r', 'R')),       // 19 KEY_R
    Some(('t', 'T')),       // 20 KEY_T
    Some(('y', 'Y')),       // 21 KEY_Y
    Some(('u', 'U')),       // 22 KEY_U
    Some(('i', 'I')),       // 23 KEY_I
    Some(('o', 'O')),       // 24 KEY_O
    Some(('p', 'P')),       // 25 KEY_P
    Some(('[', '{')),       // 26 KEY_LEFTBRACE
    Some((']', '}')),       // 27 KEY_RIGHTBRACE
    Some(('\n', '\n')),     // 28 KEY_ENTER
    None,                   // 29 KEY_LEFTCTRL
    Some(('a', 'A')),       // 30 KEY_A
    Some(('s', 'S')),       // 31 KEY_S
    Some(('d', 'D')),       // 32 KEY_D
    Some(('f', 'F')),       // 33 KEY_F
    Some(('g', 'G')),       // 34 KEY_G
    Some(('h', 'H')),       // 35 KEY_H
    Some(('j', 'J')),       // 36 KEY_J
    Some(('k', 'K')),       // 37 KEY_K
    Some(('l', 'L')),       // 38 KEY_L
    Some((';', ':')),       // 39 KEY_SEMICOLON
    Some(('\'', '"')),      // 40 KEY_APOSTROPHE
    Some(('`', '~')),       // 41 KEY_GRAVE
    None,                   // 42 KEY_LEFTSHIFT
    Some(('\\', '|')),      // 43 KEY_BACKSLASH
    Some(('z', 'Z')),       // 44 KEY_Z
    Some(('x', 'X')),       // 45 KEY_X
    Some(('c', 'C')),       // 46 KEY_C
    Some(('v', 'V')),       // 47 KEY_V
    Some(('b', 'B')),       // 48 KEY_B
    Some(('n', 'N')),       // 49 KEY_N
    Some(('m', 'M')),       // 50 KEY_M
    Some((',', '<')),       // 51 KEY_COMMA
    Some(('.', '>')),       // 52 KEY_DOT
    Some(('/', '?')),       // 53 KEY_SLASH
    None,                   // 54 KEY_RIGHTSHIFT
    Some(('*', '*')),       // 55 KEY_KPASTERISK
    None,                   // 56 KEY_LEFTALT
    Some((' ', ' ')),       // 57 KEY_SPACE
    None,                   // 58 KEY_CAPSLOCK
    None,                   // 59 KEY_F1
    None,                   // 60 KEY_F2
    None,                   // 61 KEY_F3
    None,                   // 62 KEY_F4
    None,                   // 63 KEY_F5
    None,                   // 64 KEY_F6
    None,                   // 65 KEY_F7
    None,                   // 66 KEY_F8
    None,                   // 67 KEY_F9
    None,                   // 68 KEY_F10
    None,                   // 69 KEY_NUMLOCK
    None,                   // 70 KEY_SCROLLLOCK
    Some(('7', '7')),       // 71 KEY_KP7
    Some(('8', '8')),       // 72 KEY_KP8
    Some(('9', '9')),       // 73 KEY_KP9
    Some(('-', '-')),       // 74 KEY_KPMINUS
    Some(('4', '4')),       // 75 KEY_KP4
    Some(('5', '5')),       // 76 KEY_KP5
    Some(('6', '6')),       // 77 KEY_KP6
    Some((('+'), '+')),     // 78 KEY_KPPLUS
    Some(('1', '1')),       // 79 KEY_KP1
    Some(('2', '2')),       // 80 KEY_KP2
    Some(('3', '3')),       // 81 KEY_KP3
    Some(('0', '0')),       // 82 KEY_KP0
    Some(('.', '.')),       // 83 KEY_KPDOT
];

pub fn keycode_to_char(keycode: u8, shift: bool) -> Option<char> {
    let entry = KEY_TO_CHAR.get(keycode as usize)?.as_ref()?;
    Some(if shift { entry.1 } else { entry.0 })
}

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct KeyboardEvent {
    pub state: u8,
    pub _pad1: u8,
    pub key: u16,
    pub mods: u16,
    pub _pad2: u16,
    pub unicode: u32,
}
