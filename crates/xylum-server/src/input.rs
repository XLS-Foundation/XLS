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

use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use xylum_protocol::ServerEvent;

use crate::keymap;
use crate::state::{ClientConn, ServerState, RESOURCE_ID_MASK, ROOT_WINDOW};

pub type Delivery = (Arc<ClientConn>, Vec<u8>);

pub fn key_event(
    state: &mut ServerState,
    keycode: u8,
    pressed: bool,
    repeat: bool,
) -> Vec<Delivery> {
    let is_modifier =
        keymap::modifier_mask(keycode) != 0 || keycode == keymap::CAPS_LOCK_KEYCODE;
    if repeat && is_modifier {
        return Vec::new();
    }

    let modifiers_before = state.modifiers;
    state.update_modifiers(keycode, pressed);

    let (pointer_x, pointer_y) = state.pointer;
    let start = if state.focus > 1 && state.windows.contains_key(&state.focus) {
        state.focus
    } else {
        state.window_at(pointer_x as i32, pointer_y as i32)
    };

    let mask_bit = if pressed { 1u32 } else { 2u32 };
    let mut current = start;
    let mut child = 0u32;
    loop {
        let Some(window) = state.windows.get(&current) else {
            return Vec::new();
        };
        if window.event_mask & mask_bit != 0 {
            break;
        }
        if window.parent == 0 {
            return Vec::new();
        }
        child = current;
        current = window.parent;
    }

    let Some(conn) = state.clients.get(&(current & !RESOURCE_ID_MASK)).cloned() else {
        return Vec::new();
    };
    let (origin_x, origin_y) = state.origin(current);

    let event = ServerEvent::Key {
        press: pressed,
        sequence: conn.sequence.load(Ordering::Relaxed),
        keycode,
        time: state.started.elapsed().as_millis() as u32,
        root: ROOT_WINDOW,
        event: current,
        child,
        root_x: pointer_x,
        root_y: pointer_y,
        event_x: (pointer_x as i32 - origin_x) as i16,
        event_y: (pointer_y as i32 - origin_y) as i16,
        state: modifiers_before,
    }
    .encode();

    vec![(conn, event)]
}

pub fn send(deliveries: Vec<Delivery>) {
    for (conn, bytes) in deliveries {
        let mut writer = conn.writer.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writer.write_all(&bytes);
    }
}
