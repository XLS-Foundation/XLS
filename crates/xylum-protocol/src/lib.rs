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

use std::io::{self, Read};

pub const X11_VERSION_MAJOR: u16 = 11;
pub const X11_VERSION_MINOR: u16 = 0;
pub const AUTH_NAME: &[u8] = b"MIT-MAGIC-COOKIE-1";

pub const EVENT_MASK_EXPOSURE: u32 = 1 << 15;
pub const EVENT_MASK_STRUCTURE_NOTIFY: u32 = 1 << 17;

pub const ERROR_REQUEST: u8 = 1;
pub const ERROR_VALUE: u8 = 2;
pub const ERROR_WINDOW: u8 = 3;
pub const ERROR_ATOM: u8 = 5;
pub const ERROR_FONT: u8 = 7;
pub const ERROR_MATCH: u8 = 8;
pub const ERROR_DRAWABLE: u8 = 9;
pub const ERROR_ALLOC: u8 = 11;
pub const ERROR_ID_CHOICE: u8 = 14;
pub const ERROR_NAME: u8 = 15;
pub const ERROR_LENGTH: u8 = 16;
pub const ERROR_IMPLEMENTATION: u8 = 17;

pub const SHM_MAJOR_OPCODE: u8 = 130;

const REQUEST_NAMES: &[&str] = &[
    "",
    "CreateWindow",
    "ChangeWindowAttributes",
    "GetWindowAttributes",
    "DestroyWindow",
    "DestroySubwindows",
    "ChangeSaveSet",
    "ReparentWindow",
    "MapWindow",
    "MapSubwindows",
    "UnmapWindow",
    "UnmapSubwindows",
    "ConfigureWindow",
    "CirculateWindow",
    "GetGeometry",
    "QueryTree",
    "InternAtom",
    "GetAtomName",
    "ChangeProperty",
    "DeleteProperty",
    "GetProperty",
    "ListProperties",
    "SetSelectionOwner",
    "GetSelectionOwner",
    "ConvertSelection",
    "SendEvent",
    "GrabPointer",
    "UngrabPointer",
    "GrabButton",
    "UngrabButton",
    "ChangeActivePointerGrab",
    "GrabKeyboard",
    "UngrabKeyboard",
    "GrabKey",
    "UngrabKey",
    "AllowEvents",
    "GrabServer",
    "UngrabServer",
    "QueryPointer",
    "GetMotionEvents",
    "TranslateCoordinates",
    "WarpPointer",
    "SetInputFocus",
    "GetInputFocus",
    "QueryKeymap",
    "OpenFont",
    "CloseFont",
    "QueryFont",
    "QueryTextExtents",
    "ListFonts",
    "ListFontsWithInfo",
    "SetFontPath",
    "GetFontPath",
    "CreatePixmap",
    "FreePixmap",
    "CreateGC",
    "ChangeGC",
    "CopyGC",
    "SetDashes",
    "SetClipRectangles",
    "FreeGC",
    "ClearArea",
    "CopyArea",
    "CopyPlane",
    "PolyPoint",
    "PolyLine",
    "PolySegment",
    "PolyRectangle",
    "PolyArc",
    "FillPoly",
    "PolyFillRectangle",
    "PolyFillArc",
    "PutImage",
    "GetImage",
    "PolyText8",
    "PolyText16",
    "ImageText8",
    "ImageText16",
    "CreateColormap",
    "FreeColormap",
    "CopyColormapAndFree",
    "InstallColormap",
    "UninstallColormap",
    "ListInstalledColormaps",
    "AllocColor",
    "AllocNamedColor",
    "AllocColorCells",
    "AllocColorPlanes",
    "FreeColors",
    "StoreColors",
    "StoreNamedColor",
    "QueryColors",
    "LookupColor",
    "CreateCursor",
    "CreateGlyphCursor",
    "FreeCursor",
    "RecolorCursor",
    "QueryBestSize",
    "QueryExtension",
    "ListExtensions",
    "ChangeKeyboardMapping",
    "GetKeyboardMapping",
    "ChangeKeyboardControl",
    "GetKeyboardControl",
    "Bell",
    "ChangePointerControl",
    "GetPointerControl",
    "SetScreenSaver",
    "GetScreenSaver",
    "ChangeHosts",
    "ListHosts",
    "SetAccessControl",
    "SetCloseDownMode",
    "KillClient",
    "RotateProperties",
    "ForceScreenSaver",
    "SetPointerMapping",
    "GetPointerMapping",
    "SetModifierMapping",
    "GetModifierMapping",
];

pub fn request_name(opcode: u8) -> &'static str {
    match REQUEST_NAMES.get(opcode as usize) {
        Some(name) if !name.is_empty() => *name,
        _ => "UnknownOrExtension",
    }
}

pub fn has_reply(opcode: u8) -> bool {
    matches!(
        opcode,
        3 | 14 | 15 | 16 | 17 | 20 | 21 | 23 | 26 | 31 | 38 | 39 | 40 | 43 | 44 | 47 | 48 | 49
            | 50 | 52 | 73 | 83 | 84 | 85 | 86 | 87 | 91 | 92 | 97 | 98 | 99 | 101 | 103 | 106
            | 108 | 110 | 116 | 117 | 118 | 119
    )
}

fn pad4(len: usize) -> usize {
    (4 - len % 4) % 4
}

struct Out(Vec<u8>);

impl Out {
    fn new() -> Self {
        Out(Vec::new())
    }

    fn u8(&mut self, value: u8) -> &mut Self {
        self.0.push(value);
        self
    }

    fn u16(&mut self, value: u16) -> &mut Self {
        self.0.extend_from_slice(&value.to_le_bytes());
        self
    }

    fn i16(&mut self, value: i16) -> &mut Self {
        self.0.extend_from_slice(&value.to_le_bytes());
        self
    }

    fn u32(&mut self, value: u32) -> &mut Self {
        self.0.extend_from_slice(&value.to_le_bytes());
        self
    }

    fn zeros(&mut self, count: usize) -> &mut Self {
        let new_len = self.0.len() + count;
        self.0.resize(new_len, 0);
        self
    }

    fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.0.extend_from_slice(value);
        self
    }

    fn pad(&mut self) -> &mut Self {
        let count = pad4(self.0.len());
        self.zeros(count)
    }
}

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    pub fn take(&mut self, count: usize) -> io::Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(count)
            .filter(|end| *end <= self.buf.len())
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "short request"))?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub fn skip(&mut self, count: usize) -> io::Result<()> {
        self.take(count).map(|_| ())
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> io::Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> io::Result<i16> {
        let b = self.take(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> io::Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

#[derive(Debug)]
pub struct SetupRequest {
    pub byte_order: u8,
    pub protocol_major: u16,
    pub protocol_minor: u16,
    pub auth_name: Vec<u8>,
    pub auth_data: Vec<u8>,
}

impl SetupRequest {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut head = [0u8; 12];
        reader.read_exact(&mut head)?;

        let byte_order = head[0];
        if byte_order != b'l' {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported byte order: {byte_order:#x}"),
            ));
        }

        let protocol_major = u16::from_le_bytes([head[2], head[3]]);
        let protocol_minor = u16::from_le_bytes([head[4], head[5]]);
        let auth_name_len = u16::from_le_bytes([head[6], head[7]]) as usize;
        let auth_data_len = u16::from_le_bytes([head[8], head[9]]) as usize;

        let auth_name = read_padded(reader, auth_name_len)?;
        let auth_data = read_padded(reader, auth_data_len)?;

        Ok(SetupRequest {
            byte_order,
            protocol_major,
            protocol_minor,
            auth_name,
            auth_data,
        })
    }

    pub fn is_byte_order_ok(&self) -> bool {
        self.byte_order == b'l'
    }
}

fn read_padded<R: Read>(reader: &mut R, len: usize) -> io::Result<Vec<u8>> {
    let mut data = vec![0u8; len];
    reader.read_exact(&mut data)?;
    let mut pad = vec![0u8; pad4(len)];
    reader.read_exact(&mut pad)?;
    Ok(data)
}

pub fn encode_setup_failure(reason: &str) -> Vec<u8> {
    let bytes = reason.as_bytes();
    let padded = bytes.len() + pad4(bytes.len());
    let mut out = Out::new();
    out.u8(0)
        .u8(bytes.len() as u8)
        .u16(X11_VERSION_MAJOR)
        .u16(X11_VERSION_MINOR)
        .u16((padded / 4) as u16)
        .bytes(bytes)
        .pad();
    out.0
}

#[derive(Debug)]
pub struct SetupSuccess {
    pub release_number: u32,
    pub resource_id_base: u32,
    pub resource_id_mask: u32,
    pub motion_buffer_size: u32,
    pub max_request_length: u16,
    pub image_byte_order: u8,
    pub bitmap_format_bit_order: u8,
    pub bitmap_format_scanline_unit: u8,
    pub bitmap_format_scanline_pad: u8,
    pub min_keycode: u8,
    pub max_keycode: u8,
    pub root_window: u32,
    pub root_visual: u32,
    pub default_colormap: u32,
    pub screen_width: u16,
    pub screen_height: u16,
}

impl SetupSuccess {
    pub fn encode(&self) -> Vec<u8> {
        let vendor: &[u8] = b"Xylum";
        let width_mm = (self.screen_width as u32 * 254 / 960) as u16;
        let height_mm = (self.screen_height as u32 * 254 / 960) as u16;

        let mut body = Out::new();
        body.u32(self.release_number)
            .u32(self.resource_id_base)
            .u32(self.resource_id_mask)
            .u32(self.motion_buffer_size)
            .u16(vendor.len() as u16)
            .u16(self.max_request_length)
            .u8(1)
            .u8(2)
            .u8(self.image_byte_order)
            .u8(self.bitmap_format_bit_order)
            .u8(self.bitmap_format_scanline_unit)
            .u8(self.bitmap_format_scanline_pad)
            .u8(self.min_keycode)
            .u8(self.max_keycode)
            .zeros(4)
            .bytes(vendor)
            .pad();

        body.u8(1).u8(1).u8(32).zeros(5);
        body.u8(24).u8(32).u8(32).zeros(5);

        body.u32(self.root_window)
            .u32(self.default_colormap)
            .u32(0x00ff_ffff)
            .u32(0)
            .u32(0)
            .u16(self.screen_width)
            .u16(self.screen_height)
            .u16(width_mm)
            .u16(height_mm)
            .u16(1)
            .u16(1)
            .u32(self.root_visual)
            .u8(0)
            .u8(0)
            .u8(24)
            .u8(2);

        body.u8(1).u8(0).u16(0).zeros(4);

        body.u8(24).u8(0).u16(1).zeros(4);
        body.u32(self.root_visual)
            .u8(4)
            .u8(8)
            .u16(256)
            .u32(0x00ff_0000)
            .u32(0x0000_ff00)
            .u32(0x0000_00ff)
            .zeros(4);

        let mut out = Out::new();
        out.u8(1)
            .u8(0)
            .u16(X11_VERSION_MAJOR)
            .u16(X11_VERSION_MINOR)
            .u16((body.0.len() / 4) as u16)
            .bytes(&body.0);
        out.0
    }
}

#[derive(Debug)]
pub struct RawRequest {
    pub opcode: u8,
    pub data: u8,
    pub body: Vec<u8>,
}

impl RawRequest {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Option<Self>> {
        let mut header = [0u8; 4];
        match reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let length = u16::from_le_bytes([header[2], header[3]]) as usize;
        if length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "big requests are not supported",
            ));
        }

        let mut body = vec![0u8; length * 4 - 4];
        reader.read_exact(&mut body)?;

        Ok(Some(RawRequest {
            opcode: header[0],
            data: header[1],
            body,
        }))
    }
}

#[derive(Debug, Default, Clone)]
pub struct WindowValues {
    pub background_pixel: Option<u32>,
    pub override_redirect: Option<bool>,
    pub event_mask: Option<u32>,
}

#[derive(Debug, Default, Clone)]
pub struct ConfigureValues {
    pub x: Option<i16>,
    pub y: Option<i16>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub border_width: Option<u16>,
}

fn parse_window_values(mask: u32, reader: &mut Reader) -> io::Result<WindowValues> {
    let mut values = WindowValues::default();
    for bit in 0..15u32 {
        if mask & (1u32 << bit) == 0 {
            continue;
        }
        let value = reader.u32()?;
        match bit {
            1 => values.background_pixel = Some(value),
            9 => values.override_redirect = Some(value != 0),
            11 => values.event_mask = Some(value),
            _ => {}
        }
    }
    Ok(values)
}

fn parse_configure_values(mask: u32, reader: &mut Reader) -> io::Result<ConfigureValues> {
    let mut values = ConfigureValues::default();
    for bit in 0..7u32 {
        if mask & (1u32 << bit) == 0 {
            continue;
        }
        let value = reader.u32()?;
        match bit {
            0 => values.x = Some(value as u16 as i16),
            1 => values.y = Some(value as u16 as i16),
            2 => values.width = Some(value as u16),
            3 => values.height = Some(value as u16),
            4 => values.border_width = Some(value as u16),
            _ => {}
        }
    }
    Ok(values)
}

fn read_points(reader: &mut Reader, relative: bool) -> io::Result<Vec<(i16, i16)>> {
    let mut points: Vec<(i16, i16)> = Vec::new();
    while reader.remaining() >= 4 {
        let x = reader.i16()?;
        let y = reader.i16()?;
        let point = match points.last() {
            Some(previous) if relative => {
                (previous.0.wrapping_add(x), previous.1.wrapping_add(y))
            }
            _ => (x, y),
        };
        points.push(point);
    }
    Ok(points)
}

#[derive(Debug, Clone, Copy)]
pub struct ArcSpec {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub angle1: i16,
    pub angle2: i16,
}

#[derive(Debug, Default, Clone)]
pub struct GcValues {
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub line_width: Option<u16>,
    pub font: Option<u32>,
}

fn parse_gc_values(mask: u32, reader: &mut Reader) -> io::Result<GcValues> {
    let mut values = GcValues::default();
    for bit in 0..23u32 {
        if mask & (1u32 << bit) == 0 {
            continue;
        }
        let value = reader.u32()?;
        match bit {
            2 => values.foreground = Some(value),
            3 => values.background = Some(value),
            4 => values.line_width = Some(value as u16),
            14 => values.font = Some(value),
            _ => {}
        }
    }
    Ok(values)
}

#[derive(Debug)]
pub enum ClientRequest {
    CreateWindow {
        depth: u8,
        wid: u32,
        parent: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        border_width: u16,
        class: u16,
        visual: u32,
        values: WindowValues,
    },
    ChangeWindowAttributes { window: u32, values: WindowValues },
    GetWindowAttributes { window: u32 },
    DestroyWindow { window: u32 },
    MapWindow { window: u32 },
    UnmapWindow { window: u32 },
    ConfigureWindow { window: u32, values: ConfigureValues },
    GetGeometry { drawable: u32 },
    QueryTree { window: u32 },
    InternAtom { only_if_exists: bool, name: Vec<u8> },
    GetAtomName { atom: u32 },
    ChangeProperty {
        mode: u8,
        window: u32,
        property: u32,
        type_: u32,
        format: u8,
        data: Vec<u8>,
    },
    DeleteProperty { window: u32, property: u32 },
    GetProperty {
        delete: bool,
        window: u32,
        property: u32,
        type_: u32,
        long_offset: u32,
        long_length: u32,
    },
    QueryPointer { window: u32 },
    GetInputFocus,
    QueryExtension { name: Vec<u8> },
    ListExtensions,
    GetKeyboardMapping { first_keycode: u8, count: u8 },
    GetModifierMapping,
    AllocColor {
        colormap: u32,
        red: u16,
        green: u16,
        blue: u16,
    },
    QueryColors { colormap: u32, pixels: Vec<u32> },
    LookupColor { colormap: u32, name: Vec<u8> },
    AllocNamedColor { colormap: u32, name: Vec<u8> },
    CreateGC {
        gc: u32,
        drawable: u32,
        values: GcValues,
    },
    ChangeGC { gc: u32, values: GcValues },
    FreeGC { gc: u32 },
    ClearArea {
        window: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
    PolyFillRectangle {
        drawable: u32,
        gc: u32,
        rects: Vec<(i16, i16, u16, u16)>,
    },
    PolyFillArc {
        drawable: u32,
        gc: u32,
        arcs: Vec<ArcSpec>,
    },
    PolyArc {
        drawable: u32,
        gc: u32,
        arcs: Vec<ArcSpec>,
    },
    MapSubwindows { window: u32 },
    SetInputFocus { focus: u32, revert_to: u8 },
    CreatePixmap {
        depth: u8,
        pid: u32,
        drawable: u32,
        width: u16,
        height: u16,
    },
    FreePixmap { pid: u32 },
    PutImage {
        format: u8,
        drawable: u32,
        gc: u32,
        width: u16,
        height: u16,
        dst_x: i16,
        dst_y: i16,
        left_pad: u8,
        depth: u8,
        data: Vec<u8>,
    },
    GetImage {
        format: u8,
        drawable: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
    CopyPlane {
        src: u32,
        dst: u32,
        gc: u32,
        src_x: i16,
        src_y: i16,
        dst_x: i16,
        dst_y: i16,
        width: u16,
        height: u16,
        plane: u32,
    },
    ListFontsWithInfo { max_names: u16, pattern: Vec<u8> },
    GrabPointer,
    GrabKeyboard,
    SetSelectionOwner { owner: u32, selection: u32 },
    GetSelectionOwner { selection: u32 },
    ConvertSelection {
        requestor: u32,
        selection: u32,
        target: u32,
        property: u32,
    },
    SendEvent { destination: u32, event: [u8; 32] },
    PolyPoint {
        drawable: u32,
        gc: u32,
        points: Vec<(i16, i16)>,
    },
    PolyLine {
        drawable: u32,
        gc: u32,
        points: Vec<(i16, i16)>,
    },
    PolySegment {
        drawable: u32,
        gc: u32,
        segments: Vec<(i16, i16, i16, i16)>,
    },
    PolyRectangle {
        drawable: u32,
        gc: u32,
        rects: Vec<(i16, i16, u16, u16)>,
    },
    FillPoly {
        drawable: u32,
        gc: u32,
        points: Vec<(i16, i16)>,
    },
    CopyArea {
        src: u32,
        dst: u32,
        src_x: i16,
        src_y: i16,
        dst_x: i16,
        dst_y: i16,
        width: u16,
        height: u16,
    },
    OpenFont { fid: u32, name: Vec<u8> },
    CloseFont { fid: u32 },
    QueryFont { fid: u32 },
    ListFonts { max_names: u16, pattern: Vec<u8> },
    ImageText8 {
        drawable: u32,
        gc: u32,
        x: i16,
        y: i16,
        text: Vec<u8>,
    },
    PolyText8 {
        drawable: u32,
        gc: u32,
        x: i16,
        y: i16,
        items: Vec<(i8, Vec<u8>)>,
    },
    ShmQueryVersion,
    ShmAttach {
        shmseg: u32,
        shmid: u32,
        read_only: bool,
    },
    ShmDetach {
        shmseg: u32,
    },
    ShmPutImage {
        drawable: u32,
        gc: u32,
        total_width: u16,
        total_height: u16,
        src_x: u16,
        src_y: u16,
        src_width: u16,
        src_height: u16,
        dst_x: i16,
        dst_y: i16,
        depth: u8,
        format: u8,
        send_event: bool,
        shmseg: u32,
        offset: u32,
    },
    Other { opcode: u8 },
}

impl ClientRequest {
    pub fn parse(raw: &RawRequest) -> io::Result<Self> {
        let mut r = Reader::new(&raw.body);
        let request = match raw.opcode {
            SHM_MAJOR_OPCODE => {
                let minor = raw.data;
                match minor {
                    0 => ClientRequest::ShmQueryVersion,
                    1 => {
                        let shmseg = r.u32()?;
                        let shmid = r.u32()?;
                        let read_only = r.u8()? != 0;
                        ClientRequest::ShmAttach {
                            shmseg,
                            shmid,
                            read_only,
                        }
                    }
                    2 => ClientRequest::ShmDetach {
                        shmseg: r.u32()?,
                    },
                    3 => {
                        let drawable = r.u32()?;
                        let gc = r.u32()?;
                        let total_width = r.u16()?;
                        let total_height = r.u16()?;
                        let src_x = r.u16()?;
                        let src_y = r.u16()?;
                        let src_width = r.u16()?;
                        let src_height = r.u16()?;
                        let dst_x = r.i16()?;
                        let dst_y = r.i16()?;
                        let depth = r.u8()?;
                        let format = r.u8()?;
                        let send_event = r.u8()? != 0;
                        r.skip(1)?;
                        let shmseg = r.u32()?;
                        let offset = r.u32()?;
                        ClientRequest::ShmPutImage {
                            drawable,
                            gc,
                            total_width,
                            total_height,
                            src_x,
                            src_y,
                            src_width,
                            src_height,
                            dst_x,
                            dst_y,
                            depth,
                            format,
                            send_event,
                            shmseg,
                            offset,
                        }
                    }
                    _ => ClientRequest::Other { opcode: raw.opcode },
                }
            }
            1 => {
                let wid = r.u32()?;
                let parent = r.u32()?;
                let x = r.i16()?;
                let y = r.i16()?;
                let width = r.u16()?;
                let height = r.u16()?;
                let border_width = r.u16()?;
                let class = r.u16()?;
                let visual = r.u32()?;
                let mask = r.u32()?;
                let values = parse_window_values(mask, &mut r)?;
                ClientRequest::CreateWindow {
                    depth: raw.data,
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
                }
            }
            2 => {
                let window = r.u32()?;
                let mask = r.u32()?;
                let values = parse_window_values(mask, &mut r)?;
                ClientRequest::ChangeWindowAttributes { window, values }
            }
            3 => ClientRequest::GetWindowAttributes { window: r.u32()? },
            4 => ClientRequest::DestroyWindow { window: r.u32()? },
            8 => ClientRequest::MapWindow { window: r.u32()? },
            10 => ClientRequest::UnmapWindow { window: r.u32()? },
            12 => {
                let window = r.u32()?;
                let mask = r.u16()? as u32;
                r.skip(2)?;
                let values = parse_configure_values(mask, &mut r)?;
                ClientRequest::ConfigureWindow { window, values }
            }
            14 => ClientRequest::GetGeometry { drawable: r.u32()? },
            15 => ClientRequest::QueryTree { window: r.u32()? },
            16 => {
                let name_len = r.u16()? as usize;
                r.skip(2)?;
                let name = r.take(name_len)?.to_vec();
                ClientRequest::InternAtom {
                    only_if_exists: raw.data != 0,
                    name,
                }
            }
            17 => ClientRequest::GetAtomName { atom: r.u32()? },
            18 => {
                let window = r.u32()?;
                let property = r.u32()?;
                let type_ = r.u32()?;
                let format = r.u8()?;
                r.skip(3)?;
                let len = r.u32()? as usize;
                let byte_len = len * (format as usize / 8);
                let data = r.take(byte_len)?.to_vec();
                ClientRequest::ChangeProperty {
                    mode: raw.data,
                    window,
                    property,
                    type_,
                    format,
                    data,
                }
            }
            19 => {
                let window = r.u32()?;
                let property = r.u32()?;
                ClientRequest::DeleteProperty { window, property }
            }
            20 => {
                let window = r.u32()?;
                let property = r.u32()?;
                let type_ = r.u32()?;
                let long_offset = r.u32()?;
                let long_length = r.u32()?;
                ClientRequest::GetProperty {
                    delete: raw.data != 0,
                    window,
                    property,
                    type_,
                    long_offset,
                    long_length,
                }
            }
            38 => ClientRequest::QueryPointer { window: r.u32()? },
            43 => ClientRequest::GetInputFocus,
            98 => {
                let name_len = r.u16()? as usize;
                r.skip(2)?;
                let name = r.take(name_len)?.to_vec();
                ClientRequest::QueryExtension { name }
            }
            99 => ClientRequest::ListExtensions,
            101 => {
                let first_keycode = r.u8()?;
                let count = r.u8()?;
                ClientRequest::GetKeyboardMapping {
                    first_keycode,
                    count,
                }
            }
            119 => ClientRequest::GetModifierMapping,
            84 => {
                let colormap = r.u32()?;
                let red = r.u16()?;
                let green = r.u16()?;
                let blue = r.u16()?;
                ClientRequest::AllocColor {
                    colormap,
                    red,
                    green,
                    blue,
                }
            }
            91 => {
                let colormap = r.u32()?;
                let mut pixels = Vec::new();
                while r.remaining() >= 4 {
                    pixels.push(r.u32()?);
                }
                ClientRequest::QueryColors { colormap, pixels }
            }
            85 | 92 => {
                let colormap = r.u32()?;
                let name_len = r.u16()? as usize;
                r.skip(2)?;
                let name = r.take(name_len)?.to_vec();
                if raw.opcode == 85 {
                    ClientRequest::AllocNamedColor { colormap, name }
                } else {
                    ClientRequest::LookupColor { colormap, name }
                }
            }
            55 => {
                let gc = r.u32()?;
                let drawable = r.u32()?;
                let mask = r.u32()?;
                let values = parse_gc_values(mask, &mut r)?;
                ClientRequest::CreateGC {
                    gc,
                    drawable,
                    values,
                }
            }
            56 => {
                let gc = r.u32()?;
                let mask = r.u32()?;
                let values = parse_gc_values(mask, &mut r)?;
                ClientRequest::ChangeGC { gc, values }
            }
            60 => ClientRequest::FreeGC { gc: r.u32()? },
            61 => {
                let window = r.u32()?;
                let x = r.i16()?;
                let y = r.i16()?;
                let width = r.u16()?;
                let height = r.u16()?;
                ClientRequest::ClearArea {
                    window,
                    x,
                    y,
                    width,
                    height,
                }
            }
            70 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let mut rects = Vec::new();
                while r.remaining() >= 8 {
                    let x = r.i16()?;
                    let y = r.i16()?;
                    let width = r.u16()?;
                    let height = r.u16()?;
                    rects.push((x, y, width, height));
                }
                ClientRequest::PolyFillRectangle {
                    drawable,
                    gc,
                    rects,
                }
            }
            68 | 71 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let mut arcs = Vec::new();
                while r.remaining() >= 12 {
                    let x = r.i16()?;
                    let y = r.i16()?;
                    let width = r.u16()?;
                    let height = r.u16()?;
                    let angle1 = r.i16()?;
                    let angle2 = r.i16()?;
                    arcs.push(ArcSpec {
                        x,
                        y,
                        width,
                        height,
                        angle1,
                        angle2,
                    });
                }
                if raw.opcode == 71 {
                    ClientRequest::PolyFillArc { drawable, gc, arcs }
                } else {
                    ClientRequest::PolyArc { drawable, gc, arcs }
                }
            }
            9 => ClientRequest::MapSubwindows { window: r.u32()? },
            64 | 65 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let points = read_points(&mut r, raw.data == 1)?;
                if raw.opcode == 64 {
                    ClientRequest::PolyPoint {
                        drawable,
                        gc,
                        points,
                    }
                } else {
                    ClientRequest::PolyLine {
                        drawable,
                        gc,
                        points,
                    }
                }
            }
            66 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let mut segments = Vec::new();
                while r.remaining() >= 8 {
                    let x1 = r.i16()?;
                    let y1 = r.i16()?;
                    let x2 = r.i16()?;
                    let y2 = r.i16()?;
                    segments.push((x1, y1, x2, y2));
                }
                ClientRequest::PolySegment {
                    drawable,
                    gc,
                    segments,
                }
            }
            67 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let mut rects = Vec::new();
                while r.remaining() >= 8 {
                    let x = r.i16()?;
                    let y = r.i16()?;
                    let width = r.u16()?;
                    let height = r.u16()?;
                    rects.push((x, y, width, height));
                }
                ClientRequest::PolyRectangle {
                    drawable,
                    gc,
                    rects,
                }
            }
            69 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                r.skip(1)?;
                let mode = r.u8()?;
                r.skip(2)?;
                let points = read_points(&mut r, mode == 1)?;
                ClientRequest::FillPoly {
                    drawable,
                    gc,
                    points,
                }
            }
            22 => {
                let owner = r.u32()?;
                let selection = r.u32()?;
                ClientRequest::SetSelectionOwner { owner, selection }
            }
            23 => ClientRequest::GetSelectionOwner {
                selection: r.u32()?,
            },
            24 => {
                let requestor = r.u32()?;
                let selection = r.u32()?;
                let target = r.u32()?;
                let property = r.u32()?;
                ClientRequest::ConvertSelection {
                    requestor,
                    selection,
                    target,
                    property,
                }
            }
            25 => {
                let destination = r.u32()?;
                r.skip(4)?;
                let mut event = [0u8; 32];
                event.copy_from_slice(r.take(32)?);
                ClientRequest::SendEvent { destination, event }
            }
            26 => ClientRequest::GrabPointer,
            31 => ClientRequest::GrabKeyboard,
            53 => {
                let pid = r.u32()?;
                let drawable = r.u32()?;
                let width = r.u16()?;
                let height = r.u16()?;
                ClientRequest::CreatePixmap {
                    depth: raw.data,
                    pid,
                    drawable,
                    width,
                    height,
                }
            }
            54 => ClientRequest::FreePixmap { pid: r.u32()? },
            72 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let width = r.u16()?;
                let height = r.u16()?;
                let dst_x = r.i16()?;
                let dst_y = r.i16()?;
                let left_pad = r.u8()?;
                let depth = r.u8()?;
                r.skip(2)?;
                let rest = r.remaining();
                let data = r.take(rest)?.to_vec();
                ClientRequest::PutImage {
                    format: raw.data,
                    drawable,
                    gc,
                    width,
                    height,
                    dst_x,
                    dst_y,
                    left_pad,
                    depth,
                    data,
                }
            }
            73 => {
                let drawable = r.u32()?;
                let x = r.i16()?;
                let y = r.i16()?;
                let width = r.u16()?;
                let height = r.u16()?;
                ClientRequest::GetImage {
                    format: raw.data,
                    drawable,
                    x,
                    y,
                    width,
                    height,
                }
            }
            63 => {
                let src = r.u32()?;
                let dst = r.u32()?;
                let gc = r.u32()?;
                let src_x = r.i16()?;
                let src_y = r.i16()?;
                let dst_x = r.i16()?;
                let dst_y = r.i16()?;
                let width = r.u16()?;
                let height = r.u16()?;
                let plane = r.u32()?;
                ClientRequest::CopyPlane {
                    src,
                    dst,
                    gc,
                    src_x,
                    src_y,
                    dst_x,
                    dst_y,
                    width,
                    height,
                    plane,
                }
            }
            62 => {
                let src = r.u32()?;
                let dst = r.u32()?;
                r.skip(4)?;
                let src_x = r.i16()?;
                let src_y = r.i16()?;
                let dst_x = r.i16()?;
                let dst_y = r.i16()?;
                let width = r.u16()?;
                let height = r.u16()?;
                ClientRequest::CopyArea {
                    src,
                    dst,
                    src_x,
                    src_y,
                    dst_x,
                    dst_y,
                    width,
                    height,
                }
            }
            42 => {
                let focus = r.u32()?;
                ClientRequest::SetInputFocus {
                    focus,
                    revert_to: raw.data,
                }
            }
            45 => {
                let fid = r.u32()?;
                let name_len = r.u16()? as usize;
                r.skip(2)?;
                let name = r.take(name_len)?.to_vec();
                ClientRequest::OpenFont { fid, name }
            }
            46 => ClientRequest::CloseFont { fid: r.u32()? },
            47 => ClientRequest::QueryFont { fid: r.u32()? },
            49 => {
                let max_names = r.u16()?;
                let pattern_len = r.u16()? as usize;
                let pattern = r.take(pattern_len)?.to_vec();
                ClientRequest::ListFonts { max_names, pattern }
            }
            50 => {
                let max_names = r.u16()?;
                let pattern_len = r.u16()? as usize;
                let pattern = r.take(pattern_len)?.to_vec();
                ClientRequest::ListFontsWithInfo { max_names, pattern }
            }
            76 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let x = r.i16()?;
                let y = r.i16()?;
                let text = r.take(raw.data as usize)?.to_vec();
                ClientRequest::ImageText8 {
                    drawable,
                    gc,
                    x,
                    y,
                    text,
                }
            }
            74 => {
                let drawable = r.u32()?;
                let gc = r.u32()?;
                let x = r.i16()?;
                let y = r.i16()?;
                let mut items = Vec::new();
                while r.remaining() >= 2 {
                    let len = r.u8()? as usize;
                    if len == 255 {
                        r.skip(4)?;
                        continue;
                    }
                    let delta = r.u8()? as i8;
                    let text = r.take(len)?.to_vec();
                    items.push((delta, text));
                }
                ClientRequest::PolyText8 {
                    drawable,
                    gc,
                    x,
                    y,
                    items,
                }
            }
            opcode => ClientRequest::Other { opcode },
        };
        Ok(request)
    }
}

#[derive(Debug)]
pub enum ServerReply {
    GetWindowAttributes {
        sequence: u16,
        visual: u32,
        class: u16,
        map_state: u8,
        override_redirect: bool,
        event_mask: u32,
        colormap: u32,
    },
    GetGeometry {
        sequence: u16,
        depth: u8,
        root: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        border_width: u16,
    },
    QueryTree {
        sequence: u16,
        root: u32,
        parent: u32,
        children: Vec<u32>,
    },
    InternAtom { sequence: u16, atom: u32 },
    GetAtomName { sequence: u16, name: Vec<u8> },
    GetProperty {
        sequence: u16,
        format: u8,
        type_: u32,
        bytes_after: u32,
        value_len: u32,
        data: Vec<u8>,
    },
    QueryPointer {
        sequence: u16,
        root: u32,
        root_x: i16,
        root_y: i16,
        win_x: i16,
        win_y: i16,
        mask: u16,
    },
    GrabStatus { sequence: u16, status: u8 },
    GetImage {
        sequence: u16,
        depth: u8,
        visual: u32,
        data: Vec<u8>,
    },
    GetSelectionOwner { sequence: u16, owner: u32 },
    GetInputFocus {
        sequence: u16,
        focus: u32,
        revert_to: u8,
    },
    QueryExtension {
        sequence: u16,
        present: bool,
        major_opcode: u8,
        first_event: u8,
        first_error: u8,
    },
    ListExtensions { sequence: u16, names: Vec<Vec<u8>> },
    GetKeyboardMapping {
        sequence: u16,
        keysyms_per_keycode: u8,
        keysyms: Vec<u32>,
    },
    GetModifierMapping {
        sequence: u16,
        keycodes_per_modifier: u8,
        keycodes: Vec<u8>,
    },
    AllocColor {
        sequence: u16,
        red: u16,
        green: u16,
        blue: u16,
        pixel: u32,
    },
    QueryColors {
        sequence: u16,
        colors: Vec<(u16, u16, u16)>,
    },
    LookupColor {
        sequence: u16,
        exact: (u16, u16, u16),
        visual: (u16, u16, u16),
    },
    AllocNamedColor {
        sequence: u16,
        pixel: u32,
        exact: (u16, u16, u16),
        visual: (u16, u16, u16),
    },
    QueryFont {
        sequence: u16,
        ascent: i16,
        descent: i16,
        width: i16,
        first_char: u16,
        last_char: u16,
        default_char: u16,
    },
    ListFonts {
        sequence: u16,
        names: Vec<Vec<u8>>,
    },
    ListFontsWithInfo {
        sequence: u16,
        name: Vec<u8>,
        ascent: i16,
        descent: i16,
        width: i16,
        first_char: u16,
        last_char: u16,
        default_char: u16,
    },
    ShmQueryVersion {
        sequence: u16,
    },
}

fn char_info(o: &mut Out, left: i16, right: i16, width: i16, ascent: i16, descent: i16) {
    o.i16(left)
        .i16(right)
        .i16(width)
        .i16(ascent)
        .i16(descent)
        .u16(0);
}

impl ServerReply {
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Out::new();
        match self {
            ServerReply::ShmQueryVersion { sequence } => {
                o.u8(1)
                    .u8(1)
                    .u16(*sequence)
                    .u32(0)
                    .u16(1)
                    .u16(2)
                    .u16(unsafe { libc::getuid() } as u16)
                    .u16(unsafe { libc::getgid() } as u16)
                    .u8(2)
                    .zeros(15);
            }
            ServerReply::GetWindowAttributes {
                sequence,
                visual,
                class,
                map_state,
                override_redirect,
                event_mask,
                colormap,
            } => {
                o.u8(1).u8(0).u16(*sequence).u32(3);
                o.u32(*visual).u16(*class).u8(0).u8(1);
                o.u32(0xffff_ffff).u32(0);
                o.u8(0)
                    .u8(0)
                    .u8(*map_state)
                    .u8(*override_redirect as u8);
                o.u32(*colormap).u32(*event_mask).u32(*event_mask);
                o.u16(0).u16(0);
            }
            ServerReply::GetGeometry {
                sequence,
                depth,
                root,
                x,
                y,
                width,
                height,
                border_width,
            } => {
                o.u8(1).u8(*depth).u16(*sequence).u32(0);
                o.u32(*root).i16(*x).i16(*y).u16(*width).u16(*height);
                o.u16(*border_width).zeros(10);
            }
            ServerReply::QueryTree {
                sequence,
                root,
                parent,
                children,
            } => {
                o.u8(1).u8(0).u16(*sequence).u32(children.len() as u32);
                o.u32(*root).u32(*parent).u16(children.len() as u16).zeros(14);
                for child in children {
                    o.u32(*child);
                }
            }
            ServerReply::InternAtom { sequence, atom } => {
                o.u8(1).u8(0).u16(*sequence).u32(0);
                o.u32(*atom).zeros(20);
            }
            ServerReply::GetAtomName { sequence, name } => {
                let padded = name.len() + pad4(name.len());
                o.u8(1).u8(0).u16(*sequence).u32((padded / 4) as u32);
                o.u16(name.len() as u16).zeros(22);
                o.bytes(name).pad();
            }
            ServerReply::GetProperty {
                sequence,
                format,
                type_,
                bytes_after,
                value_len,
                data,
            } => {
                let padded = data.len() + pad4(data.len());
                o.u8(1).u8(*format).u16(*sequence).u32((padded / 4) as u32);
                o.u32(*type_).u32(*bytes_after).u32(*value_len).zeros(12);
                o.bytes(data).pad();
            }
            ServerReply::QueryPointer {
                sequence,
                root,
                root_x,
                root_y,
                win_x,
                win_y,
                mask,
            } => {
                o.u8(1).u8(1).u16(*sequence).u32(0);
                o.u32(*root).u32(0);
                o.i16(*root_x).i16(*root_y).i16(*win_x).i16(*win_y);
                o.u16(*mask).zeros(6);
            }
            ServerReply::GetImage {
                sequence,
                depth,
                visual,
                data,
            } => {
                o.u8(1)
                    .u8(*depth)
                    .u16(*sequence)
                    .u32((data.len() / 4) as u32);
                o.u32(*visual).zeros(20).bytes(data);
            }
            ServerReply::GrabStatus { sequence, status } => {
                o.u8(1).u8(*status).u16(*sequence).u32(0);
                o.zeros(24);
            }
            ServerReply::GetSelectionOwner { sequence, owner } => {
                o.u8(1).u8(0).u16(*sequence).u32(0);
                o.u32(*owner).zeros(20);
            }
            ServerReply::GetInputFocus {
                sequence,
                focus,
                revert_to,
            } => {
                o.u8(1).u8(*revert_to).u16(*sequence).u32(0);
                o.u32(*focus).zeros(20);
            }
            ServerReply::QueryExtension {
                sequence,
                present,
                major_opcode,
                first_event,
                first_error,
            } => {
                o.u8(1).u8(0).u16(*sequence).u32(0);
                o.u8(*present as u8)
                    .u8(*major_opcode)
                    .u8(*first_event)
                    .u8(*first_error)
                    .zeros(20);
            }
            ServerReply::ListExtensions { sequence, names } => {
                let mut data = Out::new();
                for name in names {
                    data.u8(name.len() as u8).bytes(name);
                }
                data.pad();
                o.u8(1)
                    .u8(names.len() as u8)
                    .u16(*sequence)
                    .u32((data.0.len() / 4) as u32);
                o.zeros(24).bytes(&data.0);
            }
            ServerReply::GetKeyboardMapping {
                sequence,
                keysyms_per_keycode,
                keysyms,
            } => {
                o.u8(1)
                    .u8(*keysyms_per_keycode)
                    .u16(*sequence)
                    .u32(keysyms.len() as u32);
                o.zeros(24);
                for keysym in keysyms {
                    o.u32(*keysym);
                }
            }
            ServerReply::GetModifierMapping {
                sequence,
                keycodes_per_modifier,
                keycodes,
            } => {
                o.u8(1)
                    .u8(*keycodes_per_modifier)
                    .u16(*sequence)
                    .u32((keycodes.len() / 4) as u32);
                o.zeros(24).bytes(keycodes);
            }
            ServerReply::AllocColor {
                sequence,
                red,
                green,
                blue,
                pixel,
            } => {
                o.u8(1).u8(0).u16(*sequence).u32(0);
                o.u16(*red).u16(*green).u16(*blue).u16(0);
                o.u32(*pixel).zeros(12);
            }
            ServerReply::QueryColors { sequence, colors } => {
                o.u8(1)
                    .u8(0)
                    .u16(*sequence)
                    .u32((colors.len() * 2) as u32);
                o.u16(colors.len() as u16).zeros(22);
                for (red, green, blue) in colors {
                    o.u16(*red).u16(*green).u16(*blue).u16(0);
                }
            }
            ServerReply::LookupColor {
                sequence,
                exact,
                visual,
            } => {
                o.u8(1).u8(0).u16(*sequence).u32(0);
                o.u16(exact.0).u16(exact.1).u16(exact.2);
                o.u16(visual.0).u16(visual.1).u16(visual.2);
                o.zeros(12);
            }
            ServerReply::AllocNamedColor {
                sequence,
                pixel,
                exact,
                visual,
            } => {
                o.u8(1).u8(0).u16(*sequence).u32(0);
                o.u32(*pixel);
                o.u16(exact.0).u16(exact.1).u16(exact.2);
                o.u16(visual.0).u16(visual.1).u16(visual.2);
                o.zeros(8);
            }
            ServerReply::QueryFont {
                sequence,
                ascent,
                descent,
                width,
                first_char,
                last_char,
                default_char,
            } => {
                let count = (*last_char - *first_char + 1) as usize;
                o.u8(1)
                    .u8(0)
                    .u16(*sequence)
                    .u32((7 + 3 * count) as u32);
                char_info(&mut o, 0, *width, *width, *ascent, *descent);
                o.zeros(4);
                char_info(&mut o, 0, *width, *width, *ascent, *descent);
                o.zeros(4);
                o.u16(*first_char).u16(*last_char).u16(*default_char).u16(0);
                o.u8(0).u8(0).u8(0).u8(1);
                o.i16(*ascent).i16(*descent).u32(count as u32);
                for _ in 0..count {
                    char_info(&mut o, 0, *width, *width, *ascent, *descent);
                }
            }
            ServerReply::ListFontsWithInfo {
                sequence,
                name,
                ascent,
                descent,
                width,
                first_char,
                last_char,
                default_char,
            } => {
                let padded = name.len() + pad4(name.len());
                o.u8(1)
                    .u8(name.len() as u8)
                    .u16(*sequence)
                    .u32((7 + padded / 4) as u32);
                if name.is_empty() {
                    o.zeros(52);
                } else {
                    char_info(&mut o, 0, *width, *width, *ascent, *descent);
                    o.zeros(4);
                    char_info(&mut o, 0, *width, *width, *ascent, *descent);
                    o.zeros(4);
                    o.u16(*first_char).u16(*last_char).u16(*default_char).u16(0);
                    o.u8(0).u8(0).u8(0).u8(1);
                    o.i16(*ascent).i16(*descent).u32(0);
                    o.bytes(name).pad();
                }
            }
            ServerReply::ListFonts { sequence, names } => {
                let mut data = Out::new();
                for name in names {
                    data.u8(name.len() as u8).bytes(name);
                }
                data.pad();
                o.u8(1)
                    .u8(0)
                    .u16(*sequence)
                    .u32((data.0.len() / 4) as u32);
                o.u16(names.len() as u16).zeros(22).bytes(&data.0);
            }
        }
        o.0
    }
}

#[derive(Debug)]
pub enum ServerEvent {
    Expose {
        sequence: u16,
        window: u32,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        count: u16,
    },
    MapNotify {
        sequence: u16,
        event: u32,
        window: u32,
        override_redirect: bool,
    },
    UnmapNotify {
        sequence: u16,
        event: u32,
        window: u32,
        from_configure: bool,
    },
    ConfigureNotify {
        sequence: u16,
        event: u32,
        window: u32,
        above_sibling: u32,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        border_width: u16,
        override_redirect: bool,
    },
    Key {
        press: bool,
        sequence: u16,
        keycode: u8,
        time: u32,
        root: u32,
        event: u32,
        child: u32,
        root_x: i16,
        root_y: i16,
        event_x: i16,
        event_y: i16,
        state: u16,
    },
    Pointer {
        kind: u8,
        detail: u8,
        sequence: u16,
        time: u32,
        root: u32,
        event: u32,
        child: u32,
        root_x: i16,
        root_y: i16,
        event_x: i16,
        event_y: i16,
        state: u16,
    },
    Crossing {
        enter: bool,
        detail: u8,
        sequence: u16,
        time: u32,
        root: u32,
        event: u32,
        child: u32,
        root_x: i16,
        root_y: i16,
        event_x: i16,
        event_y: i16,
        state: u16,
        mode: u8,
        focus: bool,
    },
    SelectionClear {
        sequence: u16,
        time: u32,
        owner: u32,
        selection: u32,
    },
    SelectionRequest {
        sequence: u16,
        time: u32,
        owner: u32,
        requestor: u32,
        selection: u32,
        target: u32,
        property: u32,
    },
    SelectionNotify {
        sequence: u16,
        time: u32,
        requestor: u32,
        selection: u32,
        target: u32,
        property: u32,
    },
}

impl ServerEvent {
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Out::new();
        match self {
            ServerEvent::Expose {
                sequence,
                window,
                x,
                y,
                width,
                height,
                count,
            } => {
                o.u8(12).u8(0).u16(*sequence).u32(*window);
                o.u16(*x).u16(*y).u16(*width).u16(*height).u16(*count);
                o.zeros(14);
            }
            ServerEvent::MapNotify {
                sequence,
                event,
                window,
                override_redirect,
            } => {
                o.u8(19).u8(0).u16(*sequence).u32(*event).u32(*window);
                o.u8(*override_redirect as u8).zeros(19);
            }
            ServerEvent::UnmapNotify {
                sequence,
                event,
                window,
                from_configure,
            } => {
                o.u8(18).u8(0).u16(*sequence).u32(*event).u32(*window);
                o.u8(*from_configure as u8).zeros(19);
            }
            ServerEvent::ConfigureNotify {
                sequence,
                event,
                window,
                above_sibling,
                x,
                y,
                width,
                height,
                border_width,
                override_redirect,
            } => {
                o.u8(22).u8(0).u16(*sequence).u32(*event).u32(*window);
                o.u32(*above_sibling);
                o.i16(*x).i16(*y).u16(*width).u16(*height).u16(*border_width);
                o.u8(*override_redirect as u8).zeros(5);
            }
            ServerEvent::Key {
                press,
                sequence,
                keycode,
                time,
                root,
                event,
                child,
                root_x,
                root_y,
                event_x,
                event_y,
                state,
            } => {
                o.u8(if *press { 2 } else { 3 })
                    .u8(*keycode)
                    .u16(*sequence)
                    .u32(*time);
                o.u32(*root).u32(*event).u32(*child);
                o.i16(*root_x).i16(*root_y).i16(*event_x).i16(*event_y);
                o.u16(*state).u8(1).u8(0);
            }
            ServerEvent::Pointer {
                kind,
                detail,
                sequence,
                time,
                root,
                event,
                child,
                root_x,
                root_y,
                event_x,
                event_y,
                state,
            } => {
                o.u8(*kind).u8(*detail).u16(*sequence).u32(*time);
                o.u32(*root).u32(*event).u32(*child);
                o.i16(*root_x).i16(*root_y).i16(*event_x).i16(*event_y);
                o.u16(*state).u8(1).u8(0);
            }
            ServerEvent::Crossing {
                enter,
                detail,
                sequence,
                time,
                root,
                event,
                child,
                root_x,
                root_y,
                event_x,
                event_y,
                state,
                mode,
                focus,
            } => {
                o.u8(if *enter { 7 } else { 8 })
                    .u8(*detail)
                    .u16(*sequence)
                    .u32(*time);
                o.u32(*root).u32(*event).u32(*child);
                o.i16(*root_x).i16(*root_y).i16(*event_x).i16(*event_y);
                o.u16(*state).u8(*mode).u8(2 | *focus as u8);
            }
            ServerEvent::SelectionClear {
                sequence,
                time,
                owner,
                selection,
            } => {
                o.u8(29).u8(0).u16(*sequence).u32(*time);
                o.u32(*owner).u32(*selection).zeros(16);
            }
            ServerEvent::SelectionRequest {
                sequence,
                time,
                owner,
                requestor,
                selection,
                target,
                property,
            } => {
                o.u8(30).u8(0).u16(*sequence).u32(*time);
                o.u32(*owner).u32(*requestor).u32(*selection);
                o.u32(*target).u32(*property).zeros(4);
            }
            ServerEvent::SelectionNotify {
                sequence,
                time,
                requestor,
                selection,
                target,
                property,
            } => {
                o.u8(31).u8(0).u16(*sequence).u32(*time);
                o.u32(*requestor).u32(*selection).u32(*target).u32(*property);
                o.zeros(8);
            }
        }
        o.0
    }
}

#[derive(Debug)]
pub struct ServerError {
    pub code: u8,
    pub sequence: u16,
    pub bad_value: u32,
    pub minor_opcode: u16,
    pub major_opcode: u8,
}

impl ServerError {
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Out::new();
        o.u8(0)
            .u8(self.code)
            .u16(self.sequence)
            .u32(self.bad_value)
            .u16(self.minor_opcode)
            .u8(self.major_opcode)
            .zeros(21);
        o.0
    }
}