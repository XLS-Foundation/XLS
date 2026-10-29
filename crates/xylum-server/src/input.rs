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
use crate::state::{ClientConn, Delivery, ServerState, ROOT_WINDOW};

struct Context {
    conn: Arc<ClientConn>,
    sequence: u16,
    time: u32,
    root_x: i16,
    root_y: i16,
    event_x: i16,
    event_y: i16,
}

fn context(state: &ServerState, window: u32) -> Option<Context> {
    let conn = state.conn_for_window(window)?;
    let (origin_x, origin_y) = state.origin(window);
    let (pointer_x, pointer_y) = state.pointer;
    Some(Context {
        sequence: conn.sequence.load(Ordering::Relaxed),
        conn,
        time: state.now(),
        root_x: pointer_x,
        root_y: pointer_y,
        event_x: (pointer_x as i32 - origin_x) as i16,
        event_y: (pointer_y as i32 - origin_y) as i16,
    })
}

fn find_receiver(
    state: &ServerState,
    start: u32,
    wanted: impl Fn(u32) -> bool,
) -> Option<(u32, u32)> {
    let mut current = start;
    let mut child = 0u32;
    loop {
        let window = state.windows.get(&current)?;
        if wanted(window.event_mask) {
            return Some((current, child));
        }
        if window.parent == 0 {
            return None;
        }
        child = current;
        current = window.parent;
    }
}

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

    let before = state.modifiers | state.buttons;
    state.update_modifiers(keycode, pressed);

    let (pointer_x, pointer_y) = state.pointer;
    let start = if state.focus > 1 && state.windows.contains_key(&state.focus) {
        state.focus
    } else {
        state.window_at(pointer_x as i32, pointer_y as i32)
    };

    let mask_bit = if pressed { 1u32 } else { 2u32 };
    let Some((window, child)) = find_receiver(state, start, |mask| mask & mask_bit != 0) else {
        return Vec::new();
    };
    let Some(ctx) = context(state, window) else {
        return Vec::new();
    };

    let event = ServerEvent::Key {
        press: pressed,
        sequence: ctx.sequence,
        keycode,
        time: ctx.time,
        root: ROOT_WINDOW,
        event: window,
        child,
        root_x: ctx.root_x,
        root_y: ctx.root_y,
        event_x: ctx.event_x,
        event_y: ctx.event_y,
        state: before,
    }
    .encode();

    vec![(ctx.conn, event)]
}

fn motion_wanted(mask: u32, buttons: u16) -> bool {
    if mask & (1u32 << 6) != 0 {
        return true;
    }
    if buttons & 0x1f00 != 0 && mask & (1u32 << 13) != 0 {
        return true;
    }
    for index in 0..5u32 {
        if buttons & (0x100u16 << index) != 0 && mask & (1u32 << (8 + index)) != 0 {
            return true;
        }
    }
    false
}

fn crossing(state: &ServerState, window: u32, enter: bool, deliveries: &mut Vec<Delivery>) {
    let Some(target) = state.windows.get(&window) else {
        return;
    };
    let mask_bit = if enter { 1u32 << 4 } else { 1u32 << 5 };
    if target.event_mask & mask_bit == 0 {
        return;
    }
    let Some(ctx) = context(state, window) else {
        return;
    };
    let event = ServerEvent::Crossing {
        enter,
        detail: 3,
        sequence: ctx.sequence,
        time: ctx.time,
        root: ROOT_WINDOW,
        event: window,
        child: 0,
        root_x: ctx.root_x,
        root_y: ctx.root_y,
        event_x: ctx.event_x,
        event_y: ctx.event_y,
        state: state.modifiers | state.buttons,
        mode: 0,
        focus: state.focus <= 1 || state.focus == window,
    }
    .encode();
    deliveries.push((ctx.conn, event));
}

pub fn motion_event(state: &mut ServerState) -> Vec<Delivery> {
    let (pointer_x, pointer_y) = state.pointer;
    let deepest = state.window_at(pointer_x as i32, pointer_y as i32);
    let mut deliveries = Vec::new();

    if state.implicit_grab.is_none() && deepest != state.last_pointer_window {
        let previous = state.last_pointer_window;
        state.last_pointer_window = deepest;
        crossing(state, previous, false, &mut deliveries);
        crossing(state, deepest, true, &mut deliveries);
    }

    let start = state.implicit_grab.unwrap_or(deepest);
    let buttons = state.buttons;
    if let Some((window, child)) =
        find_receiver(state, start, |mask| motion_wanted(mask, buttons))
    {
        if let Some(ctx) = context(state, window) {
            let event = ServerEvent::Pointer {
                kind: 6,
                detail: 0,
                sequence: ctx.sequence,
                time: ctx.time,
                root: ROOT_WINDOW,
                event: window,
                child,
                root_x: ctx.root_x,
                root_y: ctx.root_y,
                event_x: ctx.event_x,
                event_y: ctx.event_y,
                state: state.modifiers | state.buttons,
            }
            .encode();
            deliveries.push((ctx.conn, event));
        }
    }

    deliveries
}

pub fn button_event(state: &mut ServerState, button: u8, pressed: bool) -> Vec<Delivery> {
    if !(1..=5).contains(&button) {
        return Vec::new();
    }
    let (pointer_x, pointer_y) = state.pointer;
    let deepest = state.window_at(pointer_x as i32, pointer_y as i32);
    let before = state.modifiers | state.buttons;
    let mask_bit = if pressed { 1u32 << 2 } else { 1u32 << 3 };
    let start = state.implicit_grab.unwrap_or(deepest);
    let receiver = find_receiver(state, start, |mask| mask & mask_bit != 0);

    let button_mask = 0x100u16 << (button - 1);
    if pressed {
        state.buttons |= button_mask;
        if state.implicit_grab.is_none() {
            state.implicit_grab = receiver.map(|(window, _)| window);
        }
    } else {
        state.buttons &= !button_mask;
        if state.buttons & 0x1f00 == 0 {
            state.implicit_grab = None;
        }
    }

    let Some((window, child)) = receiver else {
        return Vec::new();
    };
    let Some(ctx) = context(state, window) else {
        return Vec::new();
    };
    let event = ServerEvent::Pointer {
        kind: if pressed { 4 } else { 5 },
        detail: button,
        sequence: ctx.sequence,
        time: ctx.time,
        root: ROOT_WINDOW,
        event: window,
        child,
        root_x: ctx.root_x,
        root_y: ctx.root_y,
        event_x: ctx.event_x,
        event_y: ctx.event_y,
        state: before,
    }
    .encode();
    vec![(ctx.conn, event)]
}

pub fn send(deliveries: Vec<Delivery>) {
    for (conn, bytes) in deliveries {
        let mut writer = conn.writer.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writer.write_all(&bytes);
    }
}