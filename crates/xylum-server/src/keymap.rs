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
use std::process::Command;

use xkbcommon::xkb;

pub const KEYSYMS_PER_KEYCODE: u8 = 4;

pub const SHIFT: u16 = 1;
pub const LOCK: u16 = 2;
pub const CONTROL: u16 = 4;
pub const MOD1: u16 = 8;
pub const MOD2: u16 = 16;
pub const MOD3: u16 = 32;
pub const MOD4: u16 = 64;
pub const MOD5: u16 = 128;

const SHIFT_L: u32 = 0xffe1;
const SHIFT_R: u32 = 0xffe2;
const CONTROL_L: u32 = 0xffe3;
const CONTROL_R: u32 = 0xffe4;
const CAPS_LOCK: u32 = 0xffe5;
const SHIFT_LOCK: u32 = 0xffe6;
const META_L: u32 = 0xffe7;
const META_R: u32 = 0xffe8;
const ALT_L: u32 = 0xffe9;
const ALT_R: u32 = 0xffea;
const SUPER_L: u32 = 0xffeb;
const SUPER_R: u32 = 0xffec;
const HYPER_L: u32 = 0xffed;
const HYPER_R: u32 = 0xffee;
const NUM_LOCK: u32 = 0xff7f;
const MODE_SWITCH: u32 = 0xff7e;
const ISO_LEVEL3_SHIFT: u32 = 0xfe03;

struct LayoutConfig {
    layout: String,
    variant: String,
    options: Option<String>,
}

impl LayoutConfig {
    fn describe(&self) -> String {
        format!(
            "layout={} variant={} options={}",
            self.layout,
            self.variant,
            self.options.as_deref().unwrap_or("")
        )
    }
}

pub struct Keymap {
    syms: Vec<[u32; 4]>,
    modifiers: HashMap<u8, (u16, bool)>,
}

impl Keymap {
    pub fn new() -> Self {
        let config = detect_layout();
        println!("Xylum: keyboard {}", config.describe());
        if let Some(keymap) = Self::compile(&config) {
            return keymap;
        }
        eprintln!("Xylum: could not compile the keymap, falling back to us");
        let fallback = LayoutConfig {
            layout: "us".to_string(),
            variant: String::new(),
            options: None,
        };
        Self::compile(&fallback).unwrap_or_else(Self::empty)
    }

    fn empty() -> Self {
        Keymap {
            syms: vec![[0; 4]; 256],
            modifiers: HashMap::new(),
        }
    }

    fn compile(config: &LayoutConfig) -> Option<Self> {
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let compiled = xkb::Keymap::new_from_names(
            &context,
            "",
            "",
            config.layout.as_str(),
            config.variant.as_str(),
            config.options.clone(),
            xkb::COMPILE_NO_FLAGS,
        )?;

        let mut syms = vec![[0u32; 4]; 256];
        let min = compiled.min_keycode().raw();
        let max = compiled.max_keycode().raw().min(255);
        for raw in min..=max {
            let keycode = xkb::Keycode::new(raw);
            if compiled.num_layouts_for_key(keycode) == 0 {
                continue;
            }
            let levels = compiled.num_levels_for_key(keycode, 0).min(4);
            for level in 0..levels {
                if let Some(sym) = compiled.key_get_syms_by_level(keycode, 0, level).first() {
                    let mut value = sym.raw();
                    if value == ISO_LEVEL3_SHIFT {
                        value = MODE_SWITCH;
                    }
                    syms[raw as usize][level as usize] = value;
                }
            }
        }

        let mut modifiers = HashMap::new();
        for code in 8..256usize {
            let (mask, toggles) = match syms[code][0] {
                SHIFT_L | SHIFT_R => (SHIFT, false),
                CAPS_LOCK | SHIFT_LOCK => (LOCK, true),
                CONTROL_L | CONTROL_R => (CONTROL, false),
                ALT_L | ALT_R | META_L | META_R => (MOD1, false),
                NUM_LOCK => (MOD2, true),
                SUPER_L | SUPER_R | HYPER_L | HYPER_R => (MOD4, false),
                MODE_SWITCH => (MOD5, false),
                _ => continue,
            };
            modifiers.insert(code as u8, (mask, toggles));
        }

        Some(Keymap { syms, modifiers })
    }

    pub fn keysyms(&self, first_keycode: u8, count: u8) -> Vec<u32> {
        let mut keysyms = Vec::with_capacity(count as usize * 4);
        for offset in 0..count as usize {
            let entry = self
                .syms
                .get(first_keycode as usize + offset)
                .copied()
                .unwrap_or([0; 4]);
            keysyms.extend_from_slice(&entry);
        }
        keysyms
    }

    pub fn modifier(&self, keycode: u8) -> Option<(u16, bool)> {
        self.modifiers.get(&keycode).copied()
    }

    pub fn initial_modifiers(&self) -> u16 {
        if self.modifiers.values().any(|(mask, _)| *mask == MOD2) {
            MOD2
        } else {
            0
        }
    }

    pub fn modifier_keycodes(&self) -> Vec<u8> {
        let order = [SHIFT, LOCK, CONTROL, MOD1, MOD2, MOD3, MOD4, MOD5];
        let mut result = vec![0u8; 16];
        for (slot, mask) in order.iter().enumerate() {
            let mut codes: Vec<u8> = self
                .modifiers
                .iter()
                .filter(|(_, (candidate, _))| candidate == mask)
                .map(|(code, _)| *code)
                .collect();
            codes.sort_unstable();
            for (index, code) in codes.into_iter().take(2).enumerate() {
                result[slot * 2 + index] = code;
            }
        }
        result
    }
}

fn detect_layout() -> LayoutConfig {
    if let Ok(layout) = std::env::var("XYLUM_XKB_LAYOUT") {
        return LayoutConfig {
            layout,
            variant: std::env::var("XYLUM_XKB_VARIANT").unwrap_or_default(),
            options: std::env::var("XYLUM_XKB_OPTIONS").ok(),
        };
    }
    if let Ok(layout) = std::env::var("XKB_DEFAULT_LAYOUT") {
        return LayoutConfig {
            layout,
            variant: std::env::var("XKB_DEFAULT_VARIANT").unwrap_or_default(),
            options: std::env::var("XKB_DEFAULT_OPTIONS").ok(),
        };
    }
    if let Some(config) = from_setxkbmap() {
        return config;
    }
    if let Some(config) = from_xorg_conf() {
        return config;
    }
    LayoutConfig {
        layout: "us".to_string(),
        variant: String::new(),
        options: None,
    }
}

fn from_setxkbmap() -> Option<LayoutConfig> {
    let output = Command::new("setxkbmap").arg("-query").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut layout = None;
    let mut variant = String::new();
    let mut options = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "layout" => layout = Some(value),
            "variant" => variant = value,
            "options" if !value.is_empty() => options = Some(value),
            _ => {}
        }
    }
    Some(LayoutConfig {
        layout: layout?,
        variant,
        options,
    })
}

fn from_xorg_conf() -> Option<LayoutConfig> {
    let mut layout = None;
    let mut variant = String::new();
    let mut options = None;
    for entry in std::fs::read_dir("/etc/X11/xorg.conf.d").ok()?.flatten() {
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for line in text.lines() {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split('"').collect();
            if parts.len() < 4 {
                continue;
            }
            match parts[1] {
                "XkbLayout" => layout = Some(parts[3].to_string()),
                "XkbVariant" => variant = parts[3].to_string(),
                "XkbOptions" => options = Some(parts[3].to_string()),
                _ => {}
            }
        }
    }
    Some(LayoutConfig {
        layout: layout?,
        variant,
        options,
    })
}