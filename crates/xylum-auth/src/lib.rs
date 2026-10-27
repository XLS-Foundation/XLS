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

use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AuthEntry {
    pub family: u16,
    pub address: Vec<u8>,
    pub number: Vec<u8>,
    pub name: Vec<u8>,
    pub data: Vec<u8>,
}

pub fn read_xauthority(path: &Path) -> std::io::Result<Vec<AuthEntry>> {
    let content = fs::read(path)?;
    let mut entries = Vec::new();
    let mut offset = 0;

    while offset < content.len() {
        if offset + 2 > content.len() { break; }
        let family = u16::from_be_bytes([content[offset], content[offset+1]]);
        offset += 2;

        let (address, new_offset) = read_field(&content, offset)?;
        offset = new_offset;

        let (number, new_offset) = read_field(&content, offset)?;
        offset = new_offset;

        let (name, new_offset) = read_field(&content, offset)?;
        offset = new_offset;

        let (data, new_offset) = read_field(&content, offset)?;
        offset = new_offset;

        entries.push(AuthEntry { family, address, number, name, data });
    }

    Ok(entries)
}

fn read_field(data: &[u8], offset: usize) -> std::io::Result<(Vec<u8>, usize)> {
    if offset + 2 > data.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no length"));
    }
    let len = u16::from_be_bytes([data[offset], data[offset+1]]) as usize;
    let start = offset + 2;
    let end = start + len;
    if end > data.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "not enough data"));
    }
    Ok((data[start..end].to_vec(), end))
}

pub fn validate_cookie(entries: &[AuthEntry], display_number: u16, auth_name: &[u8], auth_data: &[u8]) -> bool {
    if auth_name != b"MIT-MAGIC-COOKIE-1" {
        return false;
    }

    for entry in entries {
        if entry.number != display_number.to_string().as_bytes() {
            continue;
        }
        if entry.name != b"MIT-MAGIC-COOKIE-1" {
            continue;
        }
        if entry.data == auth_data {
            return true;
        }
    }
    false
}
