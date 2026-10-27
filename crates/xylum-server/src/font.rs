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

use crate::font_data::GLYPHS;

pub const WIDTH: i32 = 8;
pub const HEIGHT: i32 = 16;
pub const ASCENT: i32 = 13;
pub const DESCENT: i32 = 3;
pub const FIRST_CHAR: u8 = 32;
pub const LAST_CHAR: u8 = 255;

const FONTS: &[(&str, bool)] = &[
    ("fixed", false),
    ("8x16", false),
    ("-misc-fixed-medium-r-normal--16-120-100-100-c-80-iso10646-1", false),
    ("-misc-fixed-medium-r-normal--16-120-100-100-c-80-iso8859-1", false),
    ("-misc-fixed-bold-r-normal--16-120-100-100-c-80-iso10646-1", true),
    ("-misc-fixed-bold-r-normal--16-120-100-100-c-80-iso8859-1", true),
];

pub fn glyph(ch: u8) -> Option<&'static [u8; 16]> {
    if ch < FIRST_CHAR {
        return None;
    }
    GLYPHS.get((ch - FIRST_CHAR) as usize)
}

pub fn lookup(name: &str) -> Option<bool> {
    FONTS
        .iter()
        .find(|(font_name, _)| font_name.eq_ignore_ascii_case(name))
        .map(|(_, bold)| *bold)
}

pub fn matching(pattern: &str) -> Vec<&'static str> {
    FONTS
        .iter()
        .filter(|(font_name, _)| glob_match(pattern, font_name))
        .map(|(font_name, _)| *font_name)
        .collect()
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let mut pi = 0;
    let mut ti = 0;
    let mut star: Option<usize> = None;
    let mut mark = 0;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}
