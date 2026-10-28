// Xylum - The X Server
// Copyright (C) 2026 Xylum Foundation
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

pub const KEYSYMS_PER_KEYCODE: u8 = 2;
pub const SHIFT: u16 = 1;
pub const LOCK: u16 = 2;
pub const CONTROL: u16 = 4;
pub const MOD1: u16 = 8;
pub const MOD4: u16 = 64;
pub const CAPS_LOCK_KEYCODE: u8 = 66;

pub struct Keymap {
    syms: [[u32; 2]; 256],
}

impl Keymap {
    pub fn new() -> Self {
        let mut keymap = Keymap {
            syms: [[0; 2]; 256],
        };
        keymap.fill();
        keymap
    }

    fn set(&mut self, evdev: usize, plain: u32, shifted: u32) {
        self.syms[evdev + 8] = [plain, shifted];
    }

    fn set_chars(&mut self, evdev: usize, plain: char, shifted: char) {
        self.set(evdev, plain as u32, shifted as u32);
    }

    fn fill(&mut self) {
        self.set(1, 0xff1b, 0xff1b);

        for (index, (plain, shifted)) in "1234567890".chars().zip("!@#$%^&*()".chars()).enumerate() {
            self.set_chars(2 + index, plain, shifted);
        }

        self.set_chars(12, '-', '_');
        self.set_chars(13, '=', '+');
        self.set(14, 0xff08, 0xff08);
        self.set(15, 0xff09, 0xfe20);

        for (base, row) in [(16usize, "qwertyuiop"), (30, "asdfghjkl"), (44, "zxcvbnm")] {
            for (index, plain) in row.chars().enumerate() {
                self.set_chars(base + index, plain, plain.to_ascii_uppercase());
            }
        }

        self.set_chars(26, '[', '{');
        self.set_chars(27, ']', '}');
        self.set(28, 0xff0d, 0xff0d);
        self.set(29, 0xffe3, 0xffe3);
        self.set_chars(39, ';', ':');
        self.set_chars(40, '\'', '"');
        self.set_chars(41, '`', '~');
        self.set(42, 0xffe1, 0xffe1);
        self.set_chars(43, '\\', '|');
        self.set_chars(51, ',', '<');
        self.set_chars(52, '.', '>');
        self.set_chars(53, '/', '?');
        self.set(54, 0xffe2, 0xffe2);
        self.set(56, 0xffe9, 0xffe9);
        self.set_chars(57, ' ', ' ');
        self.set(58, 0xffe5, 0xffe5);

        for index in 0..10usize {
            let keysym = 0xffbe + index as u32;
            self.set(59 + index, keysym, keysym);
        }

        self.set(69, 0xff7f, 0xff7f);
        self.set(70, 0xff14, 0xff14);
        self.set_chars(86, '<', '>');
        self.set(87, 0xffc8, 0xffc8);
        self.set(88, 0xffc9, 0xffc9);
        self.set(97, 0xffe4, 0xffe4);
        self.set(100, 0xffea, 0xffea);
        self.set(102, 0xff50, 0xff50);
        self.set(103, 0xff52, 0xff52);
        self.set(104, 0xff55, 0xff55);
        self.set(105, 0xff51, 0xff51);
        self.set(106, 0xff53, 0xff53);
        self.set(107, 0xff57, 0xff57);
        self.set(108, 0xff54, 0xff54);
        self.set(109, 0xff56, 0xff56);
        self.set(110, 0xff63, 0xff63);
        self.set(111, 0xffff, 0xffff);
        self.set(125, 0xffeb, 0xffeb);
        self.set(126, 0xffec, 0xffec);
    }

    pub fn keysyms(&self, first_keycode: u8, count: u8) -> Vec<u32> {
        let mut keysyms = Vec::with_capacity(count as usize * 2);
        for offset in 0..count as usize {
            let entry = self
                .syms
                .get(first_keycode as usize + offset)
                .copied()
                .unwrap_or([0, 0]);
            keysyms.push(entry[0]);
            keysyms.push(entry[1]);
        }
        keysyms
    }
}

pub fn modifier_mask(keycode: u8) -> u16 {
    match keycode {
        50 | 62 => SHIFT,
        37 | 105 => CONTROL,
        64 | 108 => MOD1,
        133 | 134 => MOD4,
        _ => 0,
    }
}

pub fn modifier_keycodes() -> Vec<u8> {
    vec![50, 62, 66, 0, 37, 105, 64, 108, 77, 0, 0, 0, 133, 134, 0, 0]
}
