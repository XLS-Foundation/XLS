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

use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::atomic::AtomicU16;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::gfx::{Clip, Framebuffer};
use crate::keymap::{self, Keymap};

pub const ROOT_WINDOW: u32 = 1;
pub const ROOT_VISUAL: u32 = 0x20;
pub const DEFAULT_COLORMAP: u32 = 0x22;
pub const SCREEN_WIDTH: u16 = 1024;
pub const SCREEN_HEIGHT: u16 = 640;
pub const ROOT_BACKGROUND: u32 = 0x0020_2428;
pub const SCREEN_DEPTH: u8 = 24;
pub const RESOURCE_ID_MASK: u32 = 0x001f_ffff;
pub const CLASS_INPUT_OUTPUT: u16 = 1;

const PREDEFINED_ATOMS: &[&str] = &[
    "PRIMARY",
    "SECONDARY",
    "ARC",
    "ATOM",
    "BITMAP",
    "CARDINAL",
    "COLORMAP",
    "CURSOR",
    "CUT_BUFFER0",
    "CUT_BUFFER1",
    "CUT_BUFFER2",
    "CUT_BUFFER3",
    "CUT_BUFFER4",
    "CUT_BUFFER5",
    "CUT_BUFFER6",
    "CUT_BUFFER7",
    "DRAWABLE",
    "FONT",
    "INTEGER",
    "PIXMAP",
    "POINT",
    "RECTANGLE",
    "RESOURCE_MANAGER",
    "RGB_COLOR_MAP",
    "RGB_BEST_MAP",
    "RGB_BLUE_MAP",
    "RGB_DEFAULT_MAP",
    "RGB_GRAY_MAP",
    "RGB_GREEN_MAP",
    "RGB_RED_MAP",
    "STRING",
    "VISUALID",
    "WINDOW",
    "WM_COMMAND",
    "WM_HINTS",
    "WM_CLIENT_MACHINE",
    "WM_ICON_NAME",
    "WM_ICON_SIZE",
    "WM_NAME",
    "WM_NORMAL_HINTS",
    "WM_SIZE_HINTS",
    "WM_ZOOM_HINTS",
    "MIN_SPACE",
    "NORM_SPACE",
    "MAX_SPACE",
    "END_SPACE",
    "SUPERSCRIPT_X",
    "SUPERSCRIPT_Y",
    "SUBSCRIPT_X",
    "SUBSCRIPT_Y",
    "UNDERLINE_POSITION",
    "UNDERLINE_THICKNESS",
    "STRIKEOUT_ASCENT",
    "STRIKEOUT_DESCENT",
    "ITALIC_ANGLE",
    "X_HEIGHT",
    "QUAD_WIDTH",
    "WEIGHT",
    "POINT_SIZE",
    "RESOLUTION",
    "COPYRIGHT",
    "NOTICE",
    "FONT_NAME",
    "FAMILY_NAME",
    "FULL_NAME",
    "CAP_HEIGHT",
    "WM_CLASS",
    "WM_TRANSIENT_FOR",
];

pub struct AtomTable {
    names: Vec<String>,
    ids: HashMap<String, u32>,
}

impl AtomTable {
    pub fn new() -> Self {
        let mut table = AtomTable {
            names: Vec::new(),
            ids: HashMap::new(),
        };
        for name in PREDEFINED_ATOMS {
            table.insert(name.to_string());
        }
        table
    }

    fn insert(&mut self, name: String) -> u32 {
        self.names.push(name.clone());
        let id = self.names.len() as u32;
        self.ids.insert(name, id);
        id
    }

    pub fn intern(&mut self, name: &str, only_if_exists: bool) -> u32 {
        if let Some(id) = self.ids.get(name) {
            return *id;
        }
        if only_if_exists {
            return 0;
        }
        self.insert(name.to_string())
    }

    pub fn name(&self, atom: u32) -> Option<&str> {
        if atom == 0 {
            return None;
        }
        self.names.get(atom as usize - 1).map(|s| s.as_str())
    }
}

pub struct Window {
    pub parent: u32,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub border_width: u16,
    pub depth: u8,
    pub class: u16,
    pub visual: u32,
    pub mapped: bool,
    pub override_redirect: bool,
    pub event_mask: u32,
    pub background_pixel: Option<u32>,
}

pub struct Property {
    pub type_: u32,
    pub format: u8,
    pub data: Vec<u8>,
}

pub struct ClientConn {
    pub writer: Mutex<UnixStream>,
    pub sequence: AtomicU16,
}

pub type Delivery = (Arc<ClientConn>, Vec<u8>);

pub struct Gc {
    pub foreground: u32,
    pub background: u32,
    pub line_width: u16,
    pub font: u32,
}

pub struct ServerState {
    pub windows: HashMap<u32, Window>,
    pub atoms: AtomTable,
    pub properties: HashMap<(u32, u32), Property>,
    pub gcs: HashMap<u32, Gc>,
    pub fonts: HashMap<u32, bool>,
    pub framebuffer: Framebuffer,
    pub pointer: (i16, i16),
    pub dirty: bool,
    pub clients: HashMap<u32, Arc<ClientConn>>,
    pub modifiers: u16,
    pub focus: u32,
    pub focus_revert: u8,
    pub started: Instant,
    pub keymap: Keymap,
    pub buttons: u16,
    pub last_pointer_window: u32,
    pub implicit_grab: Option<u32>,
    pub selections: HashMap<u32, u32>,
    pub pending: Vec<Delivery>,
    client_counter: u32,
}

impl ServerState {
    pub fn new() -> Self {
        let mut windows = HashMap::new();
        windows.insert(
            ROOT_WINDOW,
            Window {
                parent: 0,
                x: 0,
                y: 0,
                width: SCREEN_WIDTH,
                height: SCREEN_HEIGHT,
                border_width: 0,
                depth: SCREEN_DEPTH,
                class: CLASS_INPUT_OUTPUT,
                visual: ROOT_VISUAL,
                mapped: true,
                override_redirect: false,
                event_mask: 0,
                background_pixel: None,
            },
        );
        ServerState {
            windows,
            atoms: AtomTable::new(),
            properties: HashMap::new(),
            gcs: HashMap::new(),
            fonts: HashMap::new(),
            framebuffer: Framebuffer::new(
                SCREEN_WIDTH as usize,
                SCREEN_HEIGHT as usize,
                ROOT_BACKGROUND,
            ),
            pointer: (100, 100),
            dirty: true,
            clients: HashMap::new(),
            modifiers: 0,
            focus: 1,
            focus_revert: 0,
            started: Instant::now(),
            keymap: Keymap::new(),
            buttons: 0,
            last_pointer_window: ROOT_WINDOW,
            implicit_grab: None,
            selections: HashMap::new(),
            pending: Vec::new(),
            client_counter: 0,
        }
    }

    pub fn next_client_base(&mut self) -> u32 {
        let index = self.client_counter % 255 + 1;
        self.client_counter += 1;
        index << 21
    }

    pub fn is_viewable(&self, id: u32) -> bool {
        let mut current = id;
        while let Some(window) = self.windows.get(&current) {
            if !window.mapped {
                return false;
            }
            if window.parent == 0 {
                return true;
            }
            current = window.parent;
        }
        false
    }

    pub fn remove_window_tree(&mut self, id: u32) {
        let mut stack = vec![id];
        let mut doomed = Vec::new();
        while let Some(current) = stack.pop() {
            doomed.push(current);
            for (child_id, window) in &self.windows {
                if window.parent == current {
                    stack.push(*child_id);
                }
            }
        }
        for window_id in doomed {
            self.windows.remove(&window_id);
            self.properties.retain(|key, _| key.0 != window_id);
        }
    }

    pub fn conn_for_window(&self, window: u32) -> Option<Arc<ClientConn>> {
        self.clients.get(&(window & !RESOURCE_ID_MASK)).cloned()
    }

    pub fn now(&self) -> u32 {
        self.started.elapsed().as_millis() as u32
    }

    pub fn register_client(&mut self, base: u32, conn: Arc<ClientConn>) {
        self.clients.insert(base, conn);
    }

    pub fn update_modifiers(&mut self, keycode: u8, pressed: bool) {
        let mask = keymap::modifier_mask(keycode);
        if mask != 0 {
            if pressed {
                self.modifiers |= mask;
            } else {
                self.modifiers &= !mask;
            }
        } else if keycode == keymap::CAPS_LOCK_KEYCODE && pressed {
            self.modifiers ^= keymap::LOCK;
        }
    }

    pub fn origin(&self, id: u32) -> (i32, i32) {
        let mut x = 0i32;
        let mut y = 0i32;
        let mut current = id;
        while let Some(window) = self.windows.get(&current) {
            if window.parent == 0 {
                break;
            }
            x += window.x as i32;
            y += window.y as i32;
            current = window.parent;
        }
        (x, y)
    }

    pub fn window_at(&self, x: i32, y: i32) -> u32 {
        let mut current = ROOT_WINDOW;
        let mut origin_x = 0i32;
        let mut origin_y = 0i32;
        loop {
            let mut best: Option<(u32, i32, i32)> = None;
            for (id, window) in &self.windows {
                if window.parent != current || !window.mapped {
                    continue;
                }
                let left = origin_x + window.x as i32;
                let top = origin_y + window.y as i32;
                let inside = x >= left
                    && y >= top
                    && x < left + window.width as i32
                    && y < top + window.height as i32;
                if inside && best.map_or(true, |(best_id, _, _)| *id > best_id) {
                    best = Some((*id, left, top));
                }
            }
            match best {
                Some((id, left, top)) => {
                    current = id;
                    origin_x = left;
                    origin_y = top;
                }
                None => return current,
            }
        }
    }

    pub fn target(&self, id: u32) -> Option<(i32, i32, Clip)> {
        if !self.is_viewable(id) {
            return None;
        }
        let mut chain = Vec::new();
        let mut current = id;
        while let Some(window) = self.windows.get(&current) {
            chain.push(current);
            if window.parent == 0 {
                break;
            }
            current = window.parent;
        }
        chain.reverse();

        let mut origin_x = 0i32;
        let mut origin_y = 0i32;
        let mut clip: Clip = (
            0,
            0,
            self.framebuffer.width as i32,
            self.framebuffer.height as i32,
        );
        for window_id in chain {
            let window = &self.windows[&window_id];
            if window.parent != 0 {
                origin_x += window.x as i32;
                origin_y += window.y as i32;
            }
            clip = (
                clip.0.max(origin_x),
                clip.1.max(origin_y),
                clip.2.min(origin_x + window.width as i32),
                clip.3.min(origin_y + window.height as i32),
            );
        }
        Some((origin_x, origin_y, clip))
    }

    pub fn clear_window_area(&mut self, id: u32, x: i32, y: i32, width: i32, height: i32) {
        let Some((origin_x, origin_y, clip)) = self.target(id) else {
            return;
        };
        let Some(color) = self.windows.get(&id).and_then(|w| w.background_pixel) else {
            return;
        };
        self.framebuffer
            .fill_rect(origin_x + x, origin_y + y, width, height, clip, color);
        self.dirty = true;
    }

    pub fn erase_window(&mut self, id: u32) {
        let Some((origin_x, origin_y, clip)) = self.target(id) else {
            return;
        };
        let (width, height) = self
            .windows
            .get(&id)
            .map(|w| (w.width as i32, w.height as i32))
            .unwrap_or((0, 0));
        self.framebuffer
            .fill_rect(origin_x, origin_y, width, height, clip, ROOT_BACKGROUND);
        self.dirty = true;
    }

    pub fn drop_client(&mut self, base: u32) {
        self.clients.remove(&base);
        self.selections
            .retain(|_, window| *window & !RESOURCE_ID_MASK != base);
        if self
            .implicit_grab
            .is_some_and(|window| window & !RESOURCE_ID_MASK == base)
        {
            self.implicit_grab = None;
        }
        if self.last_pointer_window & !RESOURCE_ID_MASK == base {
            self.last_pointer_window = ROOT_WINDOW;
        }
        let owned: Vec<u32> = self
            .windows
            .keys()
            .copied()
            .filter(|id| *id & !RESOURCE_ID_MASK == base)
            .collect();
        for id in owned {
            self.erase_window(id);
        }
        self.windows
            .retain(|id, _| *id & !RESOURCE_ID_MASK != base);
        self.gcs.retain(|id, _| *id & !RESOURCE_ID_MASK != base);
        self.fonts.retain(|id, _| *id & !RESOURCE_ID_MASK != base);
        let windows = &self.windows;
        self.properties
            .retain(|key, _| windows.contains_key(&key.0));
    }
}