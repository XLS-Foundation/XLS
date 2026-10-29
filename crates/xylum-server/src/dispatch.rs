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

use xylum_protocol::{
    has_reply, request_name, ClientRequest, RawRequest, ServerError, ServerEvent, ServerReply,
    ERROR_ATOM, ERROR_DRAWABLE, ERROR_FONT, ERROR_ID_CHOICE, ERROR_IMPLEMENTATION, ERROR_LENGTH, ERROR_MATCH,
    ERROR_NAME, ERROR_VALUE, ERROR_WINDOW, EVENT_MASK_EXPOSURE, EVENT_MASK_STRUCTURE_NOTIFY,
};

use crate::font;
use crate::keymap;
use crate::state::{
    Gc, Property, ServerState, Window, CLASS_INPUT_OUTPUT, DEFAULT_COLORMAP, ROOT_WINDOW,
};

fn error(code: u8, sequence: u16, bad_value: u32, major_opcode: u8) -> Vec<u8> {
    ServerError {
        code,
        sequence,
        bad_value,
        minor_opcode: 0,
        major_opcode,
    }
    .encode()
}

fn expose(sequence: u16, window: u32, width: u16, height: u16) -> Vec<u8> {
    ServerEvent::Expose {
        sequence,
        window,
        x: 0,
        y: 0,
        width,
        height,
        count: 0,
    }
    .encode()
}

fn pixel_to_rgb(pixel: u32) -> (u16, u16, u16) {
    let red = ((pixel >> 16) & 0xff) as u16 * 257;
    let green = ((pixel >> 8) & 0xff) as u16 * 257;
    let blue = (pixel & 0xff) as u16 * 257;
    (red, green, blue)
}

fn rgb_to_pixel(red: u16, green: u16, blue: u16) -> u32 {
    (((red as u32) >> 8) << 16) | (((green as u32) >> 8) << 8) | ((blue as u32) >> 8)
}

fn lookup_color(name: &str) -> Option<(u16, u16, u16)> {
    let key: String = name
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let (red, green, blue): (u16, u16, u16) = match key.as_str() {
        "black" => (0, 0, 0),
        "white" => (255, 255, 255),
        "red" => (255, 0, 0),
        "green" => (0, 255, 0),
        "blue" => (0, 0, 255),
        "yellow" => (255, 255, 0),
        "cyan" => (0, 255, 255),
        "magenta" => (255, 0, 255),
        "gray" | "grey" => (190, 190, 190),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "orange" => (255, 165, 0),
        "pink" => (255, 192, 203),
        "brown" => (165, 42, 42),
        "purple" => (160, 32, 240),
        "navy" | "navyblue" => (0, 0, 128),
        "lightblue" => (173, 216, 230),
        "darkgreen" => (0, 100, 0),
        _ => return None,
    };
    Some((red * 257, green * 257, blue * 257))
}

fn mapped_tree(state: &ServerState, root: u32) -> Vec<u32> {
    let mut order = Vec::new();
    let mut stack = vec![root];
    while let Some(current) = stack.pop() {
        order.push(current);
        let mut children: Vec<u32> = state
            .windows
            .iter()
            .filter(|(_, w)| w.parent == current && w.mapped)
            .map(|(id, _)| *id)
            .collect();
        children.sort_unstable();
        children.reverse();
        stack.extend(children);
    }
    order
}

fn paint_and_expose(state: &mut ServerState, sequence: u16, window: u32, out: &mut Vec<Vec<u8>>) {
    if !state.is_viewable(window) {
        return;
    }
    for id in mapped_tree(state, window) {
        let Some(w) = state.windows.get(&id) else {
            continue;
        };
        if w.class != CLASS_INPUT_OUTPUT {
            continue;
        }
        let (width, height, event_mask) = (w.width, w.height, w.event_mask);
        state.clear_window_area(id, 0, 0, width as i32, height as i32);
        if event_mask & EVENT_MASK_EXPOSURE != 0 {
            out.push(expose(sequence, id, width, height));
        }
    }
}

fn map_window(state: &mut ServerState, sequence: u16, window: u32, out: &mut Vec<Vec<u8>>) {
    let Some(w) = state.windows.get_mut(&window) else {
        return;
    };
    if w.mapped {
        return;
    }
    w.mapped = true;
    let event_mask = w.event_mask;
    let override_redirect = w.override_redirect;
    if event_mask & EVENT_MASK_STRUCTURE_NOTIFY != 0 {
        out.push(
            ServerEvent::MapNotify {
                sequence,
                event: window,
                window,
                override_redirect,
            }
            .encode(),
        );
    }
    paint_and_expose(state, sequence, window, out);
}

fn gc_style(state: &ServerState, gc: u32) -> (u32, u16) {
    state
        .gcs
        .get(&gc)
        .map(|g| (g.foreground, g.line_width))
        .unwrap_or((0, 0))
}

fn draw_text(
    state: &mut ServerState,
    drawable: u32,
    gc: u32,
    x: i32,
    y: i32,
    text: &[u8],
    opaque: bool,
) {
    let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
        return;
    };
    let (foreground, background, bold) = match state.gcs.get(&gc) {
        Some(g) => (
            g.foreground,
            g.background,
            state.fonts.get(&g.font).copied().unwrap_or(false),
        ),
        None => (0, 1, false),
    };
    let top = origin_y + y - font::ASCENT;
    if opaque {
        state.framebuffer.fill_rect(
            origin_x + x,
            top,
            text.len() as i32 * font::WIDTH,
            font::HEIGHT,
            clip,
            background,
        );
    }
    for (index, ch) in text.iter().enumerate() {
        if let Some(glyph) = font::glyph(*ch) {
            state.framebuffer.draw_glyph(
                origin_x + x + index as i32 * font::WIDTH,
                top,
                glyph,
                bold,
                clip,
                foreground,
            );
        }
    }
    state.dirty = true;
}

pub fn process(state: &mut ServerState, sequence: u16, raw: &RawRequest) -> Vec<Vec<u8>> {
    let op = raw.opcode;
    let mut out: Vec<Vec<u8>> = Vec::new();

    let request = match ClientRequest::parse(raw) {
        Ok(request) => request,
        Err(_) => {
            out.push(error(ERROR_LENGTH, sequence, 0, op));
            return out;
        }
    };

    match request {
        ClientRequest::CreateWindow {
            depth,
            wid,
            parent,
            x,
            y,
            width,
            height,
            border_width,
            class,
            visual,
            values,
        } => {
            if state.windows.contains_key(&wid) {
                out.push(error(ERROR_ID_CHOICE, sequence, wid, op));
                return out;
            }
            let Some(parent_window) = state.windows.get(&parent) else {
                out.push(error(ERROR_WINDOW, sequence, parent, op));
                return out;
            };
            let window = Window {
                parent,
                x,
                y,
                width,
                height,
                border_width,
                depth: if depth == 0 { parent_window.depth } else { depth },
                class: if class == 0 { CLASS_INPUT_OUTPUT } else { class },
                visual: if visual == 0 { parent_window.visual } else { visual },
                mapped: false,
                override_redirect: values.override_redirect.unwrap_or(false),
                event_mask: values.event_mask.unwrap_or(0),
                background_pixel: values.background_pixel,
            };
            state.windows.insert(wid, window);
        }

        ClientRequest::ChangeWindowAttributes { window, values } => {
            let Some(w) = state.windows.get_mut(&window) else {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            };
            if let Some(value) = values.event_mask {
                w.event_mask = value;
            }
            if let Some(value) = values.override_redirect {
                w.override_redirect = value;
            }
            if let Some(value) = values.background_pixel {
                w.background_pixel = Some(value);
            }
        }

        ClientRequest::GetWindowAttributes { window } => {
            let Some(w) = state.windows.get(&window) else {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            };
            let map_state = if !w.mapped {
                0
            } else if state.is_viewable(window) {
                2
            } else {
                1
            };
            out.push(
                ServerReply::GetWindowAttributes {
                    sequence,
                    visual: w.visual,
                    class: w.class,
                    map_state,
                    override_redirect: w.override_redirect,
                    event_mask: w.event_mask,
                    colormap: DEFAULT_COLORMAP,
                }
                .encode(),
            );
        }

        ClientRequest::DestroyWindow { window } => {
            if window == ROOT_WINDOW {
                return out;
            }
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            state.erase_window(window);
            state.remove_window_tree(window);
        }

        ClientRequest::MapWindow { window } => {
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            map_window(state, sequence, window, &mut out);
        }

        ClientRequest::MapSubwindows { window } => {
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            let mut children: Vec<u32> = state
                .windows
                .iter()
                .filter(|(_, w)| w.parent == window)
                .map(|(id, _)| *id)
                .collect();
            children.sort_unstable();
            for child in children {
                map_window(state, sequence, child, &mut out);
            }
        }

        ClientRequest::UnmapWindow { window } => {
            let Some(w) = state.windows.get_mut(&window) else {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            };
            if !w.mapped {
                return out;
            }
            state.erase_window(window);
            let Some(w) = state.windows.get_mut(&window) else {
                return out;
            };
            w.mapped = false;
            if w.event_mask & EVENT_MASK_STRUCTURE_NOTIFY != 0 {
                out.push(
                    ServerEvent::UnmapNotify {
                        sequence,
                        event: window,
                        window,
                        from_configure: false,
                    }
                    .encode(),
                );
            }
        }

        ClientRequest::ConfigureWindow { window, values } => {
            let Some(w) = state.windows.get_mut(&window) else {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            };
            if let Some(value) = values.x {
                w.x = value;
            }
            if let Some(value) = values.y {
                w.y = value;
            }
            if let Some(value) = values.width {
                w.width = value;
            }
            if let Some(value) = values.height {
                w.height = value;
            }
            if let Some(value) = values.border_width {
                w.border_width = value;
            }
            let event_mask = w.event_mask;
            let override_redirect = w.override_redirect;
            let (x, y, width, height, border_width) =
                (w.x, w.y, w.width, w.height, w.border_width);
            let mapped = w.mapped;
            let class = w.class;
            let resized = values.width.is_some() || values.height.is_some();

            if event_mask & EVENT_MASK_STRUCTURE_NOTIFY != 0 {
                out.push(
                    ServerEvent::ConfigureNotify {
                        sequence,
                        event: window,
                        window,
                        above_sibling: 0,
                        x,
                        y,
                        width,
                        height,
                        border_width,
                        override_redirect,
                    }
                    .encode(),
                );
            }
            if resized
                && mapped
                && event_mask & EVENT_MASK_EXPOSURE != 0
                && class == CLASS_INPUT_OUTPUT
                && state.is_viewable(window)
            {
                out.push(expose(sequence, window, width, height));
            }
        }

        ClientRequest::GetGeometry { drawable } => {
            let Some(w) = state.windows.get(&drawable) else {
                out.push(error(ERROR_DRAWABLE, sequence, drawable, op));
                return out;
            };
            out.push(
                ServerReply::GetGeometry {
                    sequence,
                    depth: w.depth,
                    root: ROOT_WINDOW,
                    x: w.x,
                    y: w.y,
                    width: w.width,
                    height: w.height,
                    border_width: w.border_width,
                }
                .encode(),
            );
        }

        ClientRequest::QueryTree { window } => {
            let Some(w) = state.windows.get(&window) else {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            };
            let parent = w.parent;
            let mut children: Vec<u32> = state
                .windows
                .iter()
                .filter(|(_, candidate)| candidate.parent == window)
                .map(|(id, _)| *id)
                .collect();
            children.sort_unstable();
            out.push(
                ServerReply::QueryTree {
                    sequence,
                    root: ROOT_WINDOW,
                    parent,
                    children,
                }
                .encode(),
            );
        }

        ClientRequest::InternAtom {
            only_if_exists,
            name,
        } => {
            let name = String::from_utf8_lossy(&name).into_owned();
            let atom = state.atoms.intern(&name, only_if_exists);
            out.push(ServerReply::InternAtom { sequence, atom }.encode());
        }

        ClientRequest::GetAtomName { atom } => match state.atoms.name(atom) {
            Some(name) => {
                out.push(
                    ServerReply::GetAtomName {
                        sequence,
                        name: name.as_bytes().to_vec(),
                    }
                    .encode(),
                );
            }
            None => out.push(error(ERROR_ATOM, sequence, atom, op)),
        },

        ClientRequest::ChangeProperty {
            mode,
            window,
            property,
            type_,
            format,
            data,
        } => {
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            if !matches!(format, 8 | 16 | 32) {
                out.push(error(ERROR_VALUE, sequence, format as u32, op));
                return out;
            }
            if mode > 2 {
                out.push(error(ERROR_VALUE, sequence, mode as u32, op));
                return out;
            }
            let key = (window, property);
            if let Some(existing) = state.properties.get_mut(&key) {
                if mode == 0 {
                    *existing = Property {
                        type_,
                        format,
                        data,
                    };
                } else if existing.type_ != type_ || existing.format != format {
                    out.push(error(ERROR_MATCH, sequence, 0, op));
                } else if mode == 1 {
                    let mut merged = data;
                    merged.extend_from_slice(&existing.data);
                    existing.data = merged;
                } else {
                    existing.data.extend_from_slice(&data);
                }
            } else {
                state.properties.insert(
                    key,
                    Property {
                        type_,
                        format,
                        data,
                    },
                );
            }
        }

        ClientRequest::DeleteProperty { window, property } => {
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            state.properties.remove(&(window, property));
        }

        ClientRequest::GetProperty {
            delete,
            window,
            property,
            type_,
            long_offset,
            long_length,
        } => {
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            let key = (window, property);
            let Some(prop) = state.properties.get(&key) else {
                out.push(
                    ServerReply::GetProperty {
                        sequence,
                        format: 0,
                        type_: 0,
                        bytes_after: 0,
                        value_len: 0,
                        data: Vec::new(),
                    }
                    .encode(),
                );
                return out;
            };

            let total = prop.data.len();
            let format = prop.format;
            let actual_type = prop.type_;

            if type_ != 0 && type_ != actual_type {
                out.push(
                    ServerReply::GetProperty {
                        sequence,
                        format,
                        type_: actual_type,
                        bytes_after: total as u32,
                        value_len: 0,
                        data: Vec::new(),
                    }
                    .encode(),
                );
                return out;
            }

            let offset = long_offset as usize * 4;
            if offset > total {
                out.push(error(ERROR_VALUE, sequence, long_offset, op));
                return out;
            }
            let available = total - offset;
            let taken = available.min(long_length as usize * 4);
            let bytes_after = (available - taken) as u32;
            let data = prop.data[offset..offset + taken].to_vec();
            let value_len = (taken / (format as usize / 8)) as u32;

            out.push(
                ServerReply::GetProperty {
                    sequence,
                    format,
                    type_: actual_type,
                    bytes_after,
                    value_len,
                    data,
                }
                .encode(),
            );
            if delete && bytes_after == 0 {
                state.properties.remove(&key);
            }
        }

        ClientRequest::QueryPointer { window } => {
            if !state.windows.contains_key(&window) {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            }
            let (root_x, root_y) = state.pointer;
            let (origin_x, origin_y) = state
                .target(window)
                .map(|(x, y, _)| (x, y))
                .unwrap_or((0, 0));
            out.push(
                ServerReply::QueryPointer {
                    sequence,
                    root: ROOT_WINDOW,
                    root_x,
                    root_y,
                    win_x: (root_x as i32 - origin_x) as i16,
                    win_y: (root_y as i32 - origin_y) as i16,
                }
                .encode(),
            );
        }

        ClientRequest::GetInputFocus => {
            out.push(
                ServerReply::GetInputFocus {
                    sequence,
                    focus: state.focus,
                    revert_to: state.focus_revert,
                }
                .encode(),
            );
        }

        ClientRequest::QueryExtension { name } => {
            println!("Xylum: QueryExtension {:?}", String::from_utf8_lossy(&name));
            out.push(
                ServerReply::QueryExtension {
                    sequence,
                    present: false,
                    major_opcode: 0,
                    first_event: 0,
                    first_error: 0,
                }
                .encode(),
            );
        }

        ClientRequest::ListExtensions => {
            out.push(
                ServerReply::ListExtensions {
                    sequence,
                    names: Vec::new(),
                }
                .encode(),
            );
        }

        ClientRequest::GetKeyboardMapping {
            first_keycode,
            count,
        } => {
            if count == 0 {
                out.push(error(ERROR_VALUE, sequence, 0, op));
                return out;
            }
            out.push(
                ServerReply::GetKeyboardMapping {
                    sequence,
                    keysyms_per_keycode: keymap::KEYSYMS_PER_KEYCODE,
                    keysyms: state.keymap.keysyms(first_keycode, count),
                }
                .encode(),
            );
        }

        ClientRequest::GetModifierMapping => {
            out.push(
                ServerReply::GetModifierMapping {
                    sequence,
                    keycodes_per_modifier: 2,
                    keycodes: keymap::modifier_keycodes(),
                }
                .encode(),
            );
        }

        ClientRequest::SetInputFocus { focus, revert_to } => {
            state.focus = focus;
            state.focus_revert = revert_to;
        }

        ClientRequest::AllocColor {
            colormap: _,
            red,
            green,
            blue,
        } => {
            let pixel = rgb_to_pixel(red, green, blue);
            let (red, green, blue) = pixel_to_rgb(pixel);
            out.push(
                ServerReply::AllocColor {
                    sequence,
                    red,
                    green,
                    blue,
                    pixel,
                }
                .encode(),
            );
        }

        ClientRequest::QueryColors {
            colormap: _,
            pixels,
        } => {
            let colors = pixels.iter().map(|pixel| pixel_to_rgb(*pixel)).collect();
            out.push(ServerReply::QueryColors { sequence, colors }.encode());
        }

        ClientRequest::LookupColor { colormap: _, name } => {
            match lookup_color(&String::from_utf8_lossy(&name)) {
                Some(exact) => {
                    let visual = pixel_to_rgb(rgb_to_pixel(exact.0, exact.1, exact.2));
                    out.push(
                        ServerReply::LookupColor {
                            sequence,
                            exact,
                            visual,
                        }
                        .encode(),
                    );
                }
                None => out.push(error(ERROR_NAME, sequence, 0, op)),
            }
        }

        ClientRequest::AllocNamedColor { colormap: _, name } => {
            match lookup_color(&String::from_utf8_lossy(&name)) {
                Some(exact) => {
                    let pixel = rgb_to_pixel(exact.0, exact.1, exact.2);
                    let visual = pixel_to_rgb(pixel);
                    out.push(
                        ServerReply::AllocNamedColor {
                            sequence,
                            pixel,
                            exact,
                            visual,
                        }
                        .encode(),
                    );
                }
                None => out.push(error(ERROR_NAME, sequence, 0, op)),
            }
        }

        ClientRequest::CreateGC {
            gc,
            drawable: _,
            values,
        } => {
            state.gcs.insert(
                gc,
                Gc {
                    foreground: values.foreground.unwrap_or(0),
                    background: values.background.unwrap_or(1),
                    line_width: values.line_width.unwrap_or(0),
                    font: values.font.unwrap_or(0),
                },
            );
        }

        ClientRequest::ChangeGC { gc, values } => {
            if let Some(entry) = state.gcs.get_mut(&gc) {
                if let Some(value) = values.foreground {
                    entry.foreground = value;
                }
                if let Some(value) = values.background {
                    entry.background = value;
                }
                if let Some(value) = values.line_width {
                    entry.line_width = value;
                }
                if let Some(value) = values.font {
                    entry.font = value;
                }
            }
        }

        ClientRequest::FreeGC { gc } => {
            state.gcs.remove(&gc);
        }

        ClientRequest::ClearArea {
            window,
            x,
            y,
            width,
            height,
        } => {
            let Some(w) = state.windows.get(&window) else {
                out.push(error(ERROR_WINDOW, sequence, window, op));
                return out;
            };
            let clear_width = if width == 0 {
                (w.width as i32 - x as i32).max(0)
            } else {
                width as i32
            };
            let clear_height = if height == 0 {
                (w.height as i32 - y as i32).max(0)
            } else {
                height as i32
            };
            state.clear_window_area(window, x as i32, y as i32, clear_width, clear_height);
        }

        ClientRequest::PolyFillRectangle {
            drawable,
            gc,
            rects,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let color = state.gcs.get(&gc).map(|g| g.foreground).unwrap_or(0);
            for (x, y, width, height) in rects {
                state.framebuffer.fill_rect(
                    origin_x + x as i32,
                    origin_y + y as i32,
                    width as i32,
                    height as i32,
                    clip,
                    color,
                );
            }
            state.dirty = true;
        }

        ClientRequest::PolyFillArc {
            drawable,
            gc,
            arcs,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let color = state.gcs.get(&gc).map(|g| g.foreground).unwrap_or(0);
            for arc in &arcs {
                state.framebuffer.fill_arc(
                    origin_x + arc.x as i32,
                    origin_y + arc.y as i32,
                    arc.width as i32,
                    arc.height as i32,
                    arc.angle1,
                    arc.angle2,
                    clip,
                    color,
                );
            }
            state.dirty = true;
        }

        ClientRequest::PolyArc {
            drawable,
            gc,
            arcs,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let (color, line_width) = state
                .gcs
                .get(&gc)
                .map(|g| (g.foreground, g.line_width))
                .unwrap_or((0, 0));
            for arc in &arcs {
                state.framebuffer.stroke_arc(
                    origin_x + arc.x as i32,
                    origin_y + arc.y as i32,
                    arc.width as i32,
                    arc.height as i32,
                    arc.angle1,
                    arc.angle2,
                    line_width,
                    clip,
                    color,
                );
            }
            state.dirty = true;
        }

        ClientRequest::OpenFont { fid, name } => {
            let name = String::from_utf8_lossy(&name).into_owned();
            match font::lookup(&name) {
                Some(bold) => {
                    state.fonts.insert(fid, bold);
                }
                None => out.push(error(ERROR_NAME, sequence, 0, op)),
            }
        }

        ClientRequest::CloseFont { fid } => {
            state.fonts.remove(&fid);
        }

        ClientRequest::QueryFont { fid } => {
            if !state.fonts.contains_key(&fid) && !state.gcs.contains_key(&fid) {
                out.push(error(ERROR_FONT, sequence, fid, op));
                return out;
            }
            out.push(
                ServerReply::QueryFont {
                    sequence,
                    ascent: font::ASCENT as i16,
                    descent: font::DESCENT as i16,
                    width: font::WIDTH as i16,
                    first_char: font::FIRST_CHAR as u16,
                    last_char: font::LAST_CHAR as u16,
                    default_char: font::FIRST_CHAR as u16,
                }
                .encode(),
            );
        }

        ClientRequest::ListFonts { max_names, pattern } => {
            let pattern = String::from_utf8_lossy(&pattern).into_owned();
            let names: Vec<Vec<u8>> = font::matching(&pattern)
                .into_iter()
                .take(max_names as usize)
                .map(|name| name.as_bytes().to_vec())
                .collect();
            out.push(ServerReply::ListFonts { sequence, names }.encode());
        }

        ClientRequest::ImageText8 {
            drawable,
            gc,
            x,
            y,
            text,
        } => {
            draw_text(state, drawable, gc, x as i32, y as i32, &text, true);
        }

        ClientRequest::PolyText8 {
            drawable,
            gc,
            x,
            y,
            items,
        } => {
            let mut cursor = x as i32;
            for (delta, text) in items {
                cursor += delta as i32;
                draw_text(state, drawable, gc, cursor, y as i32, &text, false);
                cursor += text.len() as i32 * font::WIDTH;
            }
        }

        ClientRequest::CopyArea {
            src,
            dst,
            src_x,
            src_y,
            dst_x,
            dst_y,
            width,
            height,
        } => {
            let Some((src_origin_x, src_origin_y, src_clip)) = state.target(src) else {
                return out;
            };
            let Some((dst_origin_x, dst_origin_y, dst_clip)) = state.target(dst) else {
                return out;
            };
            state.framebuffer.copy_rect(
                src_origin_x + src_x as i32,
                src_origin_y + src_y as i32,
                dst_origin_x + dst_x as i32,
                dst_origin_y + dst_y as i32,
                width as i32,
                height as i32,
                src_clip,
                dst_clip,
            );
            state.dirty = true;
        }

        ClientRequest::PolyPoint {
            drawable,
            gc,
            points,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let (color, _) = gc_style(state, gc);
            for (x, y) in points {
                state
                    .framebuffer
                    .fill_rect(origin_x + x as i32, origin_y + y as i32, 1, 1, clip, color);
            }
            state.dirty = true;
        }

        ClientRequest::PolyLine {
            drawable,
            gc,
            points,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let (color, width) = gc_style(state, gc);
            for pair in points.windows(2) {
                state.framebuffer.draw_line(
                    origin_x + pair[0].0 as i32,
                    origin_y + pair[0].1 as i32,
                    origin_x + pair[1].0 as i32,
                    origin_y + pair[1].1 as i32,
                    width,
                    clip,
                    color,
                );
            }
            state.dirty = true;
        }

        ClientRequest::PolySegment {
            drawable,
            gc,
            segments,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let (color, width) = gc_style(state, gc);
            for (x1, y1, x2, y2) in segments {
                state.framebuffer.draw_line(
                    origin_x + x1 as i32,
                    origin_y + y1 as i32,
                    origin_x + x2 as i32,
                    origin_y + y2 as i32,
                    width,
                    clip,
                    color,
                );
            }
            state.dirty = true;
        }

        ClientRequest::PolyRectangle {
            drawable,
            gc,
            rects,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let (color, width) = gc_style(state, gc);
            for (x, y, w, h) in rects {
                let left = origin_x + x as i32;
                let top = origin_y + y as i32;
                let right = left + w as i32;
                let bottom = top + h as i32;
                let edges = [
                    (left, top, right, top),
                    (right, top, right, bottom),
                    (right, bottom, left, bottom),
                    (left, bottom, left, top),
                ];
                for (x0, y0, x1, y1) in edges {
                    state
                        .framebuffer
                        .draw_line(x0, y0, x1, y1, width, clip, color);
                }
            }
            state.dirty = true;
        }

        ClientRequest::FillPoly {
            drawable,
            gc,
            points,
        } => {
            let Some((origin_x, origin_y, clip)) = state.target(drawable) else {
                return out;
            };
            let (color, _) = gc_style(state, gc);
            let absolute: Vec<(i32, i32)> = points
                .iter()
                .map(|(x, y)| (origin_x + *x as i32, origin_y + *y as i32))
                .collect();
            state.framebuffer.fill_polygon(&absolute, clip, color);
            state.dirty = true;
        }

        ClientRequest::Other { opcode } => {
            if has_reply(opcode) {
                eprintln!(
                    "Xylum: unimplemented request with reply: {} ({opcode})",
                    request_name(opcode)
                );
                out.push(error(ERROR_IMPLEMENTATION, sequence, 0, opcode));
            } else {
                println!(
                    "Xylum: ignoring request without reply: {} ({opcode})",
                    request_name(opcode)
                );
            }
        }
    }

    out
}