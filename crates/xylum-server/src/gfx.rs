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

pub type Clip = (i32, i32, i32, i32);

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

fn in_sector(dx: f64, dy: f64, full: bool, start: f64, sweep: f64) -> bool {
    if full {
        return true;
    }
    let angle = (-dy).atan2(dx).to_degrees();
    if sweep >= 0.0 {
        (angle - start).rem_euclid(360.0) <= sweep
    } else {
        (start - angle).rem_euclid(360.0) <= -sweep
    }
}

impl Framebuffer {
    pub fn new(width: usize, height: usize, color: u32) -> Self {
        Framebuffer {
            width,
            height,
            pixels: vec![color; width * height],
        }
    }

    fn put(&mut self, x: i32, y: i32, clip: Clip, color: u32) {
        if x < clip.0 || y < clip.1 || x >= clip.2 || y >= clip.3 || x < 0 || y < 0 {
            return;
        }
        let (xu, yu) = (x as usize, y as usize);
        if xu < self.width && yu < self.height {
            self.pixels[yu * self.width + xu] = color;
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, clip: Clip, color: u32) {
        let x0 = x.max(clip.0).max(0);
        let y0 = y.max(clip.1).max(0);
        let x1 = x.saturating_add(w).min(clip.2).min(self.width as i32);
        let y1 = y.saturating_add(h).min(clip.3).min(self.height as i32);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for yy in y0..y1 {
            let row = yy as usize * self.width;
            self.pixels[row + x0 as usize..row + x1 as usize].fill(color);
        }
    }

    pub fn draw_glyph(
        &mut self,
        x: i32,
        y: i32,
        glyph: &[u8; 16],
        bold: bool,
        clip: Clip,
        color: u32,
    ) {
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..8i32 {
                if *bits & (0x80u8 >> col) != 0 {
                    self.put(x + col, y + row as i32, clip, color);
                    if bold {
                        self.put(x + col + 1, y + row as i32, clip, color);
                    }
                }
            }
        }
    }

    pub fn fill_arc(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        angle1: i16,
        angle2: i16,
        clip: Clip,
        color: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let rx = w as f64 / 2.0;
        let ry = h as f64 / 2.0;
        let cx = x as f64 + rx;
        let cy = y as f64 + ry;
        let full = (angle2 as i32).abs() >= 360 * 64;
        let start = angle1 as f64 / 64.0;
        let sweep = angle2 as f64 / 64.0;
        for py in y..y + h {
            for px in x..x + w {
                let dx = (px as f64 + 0.5 - cx) / rx;
                let dy = (py as f64 + 0.5 - cy) / ry;
                if dx * dx + dy * dy > 1.0 {
                    continue;
                }
                if !in_sector(dx, dy, full, start, sweep) {
                    continue;
                }
                self.put(px, py, clip, color);
            }
        }
    }

    pub fn stroke_arc(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        angle1: i16,
        angle2: i16,
        line_width: u16,
        clip: Clip,
        color: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let thickness = line_width.max(1) as f64;
        let half = thickness / 2.0;
        let rx = w as f64 / 2.0;
        let ry = h as f64 / 2.0;
        let cx = x as f64 + rx;
        let cy = y as f64 + ry;
        let outer_x = rx + half;
        let outer_y = ry + half;
        let inner_x = rx - half;
        let inner_y = ry - half;
        let full = (angle2 as i32).abs() >= 360 * 64;
        let start = angle1 as f64 / 64.0;
        let sweep = angle2 as f64 / 64.0;
        let margin = line_width.max(1) as i32;
        for py in (y - margin)..(y + h + margin) {
            for px in (x - margin)..(x + w + margin) {
                let fx = px as f64 + 0.5 - cx;
                let fy = py as f64 + 0.5 - cy;
                let outer = (fx / outer_x).powi(2) + (fy / outer_y).powi(2);
                if outer > 1.0 {
                    continue;
                }
                if inner_x > 0.0 && inner_y > 0.0 {
                    let inner = (fx / inner_x).powi(2) + (fy / inner_y).powi(2);
                    if inner < 1.0 {
                        continue;
                    }
                }
                if !in_sector(fx / outer_x, fy / outer_y, full, start, sweep) {
                    continue;
                }
                self.put(px, py, clip, color);
            }
        }
    }
}
