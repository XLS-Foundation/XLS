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

mod colors;
mod dispatch;
mod font;
mod font_data;
mod gfx;
mod input;
mod keymap;
mod presenter;
mod state;

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use xylum_auth::{read_xauthority, validate_cookie, AuthEntry};
use xylum_protocol::{
    encode_setup_failure, request_name, RawRequest, SetupRequest, SetupSuccess, AUTH_NAME,
};

use state::{
    ClientConn, ServerState, DEFAULT_COLORMAP, RESOURCE_ID_MASK, ROOT_VISUAL, ROOT_WINDOW, SCREEN_HEIGHT,
    SCREEN_WIDTH,
};

const DISPLAY_NUMBER: u16 = 2;
const SOCKET_DIR: &str = "/tmp/.X11-unix";

struct Shared {
    state: Arc<Mutex<ServerState>>,
    auth_entries: Vec<AuthEntry>,
    auth_disabled: bool,
}

fn lock(shared: &Shared) -> MutexGuard<'_, ServerState> {
    shared.state.lock().unwrap_or_else(|e| e.into_inner())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(SOCKET_DIR)?;
    let _ = std::fs::set_permissions(SOCKET_DIR, std::fs::Permissions::from_mode(0o1777));

    let path = PathBuf::from(format!("{SOCKET_DIR}/X{DISPLAY_NUMBER}"));
    let _ = std::fs::remove_file(&path);

    let auth_disabled = std::env::var_os("XYLUM_NO_AUTH").is_some();
    let auth_entries = if auth_disabled {
        println!("Xylum: authentication disabled (XYLUM_NO_AUTH is set)");
        Vec::new()
    } else {
        let auth_path = std::env::var("XAUTHORITY")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                PathBuf::from(home).join(".Xauthority")
            });
        let entries = read_xauthority(&auth_path).unwrap_or_else(|e| {
            eprintln!("Xylum: warning: could not read {auth_path:?}: {e}");
            Vec::new()
        });
        println!("Xylum: loaded {} auth entries", entries.len());
        entries
    };

    let state = Arc::new(Mutex::new(ServerState::new()));
    let shared = Arc::new(Shared {
        state: Arc::clone(&state),
        auth_entries,
        auth_disabled,
    });

    let listener = UnixListener::bind(&path)?;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o777));
    println!("Xylum: listening on {}", path.display());

    let accept_shared = Arc::clone(&shared);
    let accept_thread = std::thread::spawn(move || accept_loop(listener, accept_shared));

    if std::env::var_os("XYLUM_HEADLESS").is_some() {
        println!("Xylum: running headless (XYLUM_HEADLESS is set)");
        let _ = accept_thread.join();
    } else {
        if let Ok(display) = std::env::var("DISPLAY") {
            let own = format!(":{DISPLAY_NUMBER}");
            if display == own || display.starts_with(&format!("{own}.")) {
                eprintln!(
                    "Xylum: DISPLAY is {display}, which is Xylum itself; set DISPLAY to your desktop display (for example DISPLAY=:0) and run again"
                );
                std::process::exit(1);
            }
        }
        presenter::run(state);
    }

    Ok(())
}

fn accept_loop(listener: UnixListener, shared: Arc<Shared>) {
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || {
                    if let Err(e) = handle_client(&mut stream, &shared) {
                        eprintln!("Xylum: client error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Xylum: accept error: {e}"),
        }
    }
}

fn send_setup_failure(stream: &mut UnixStream, reason: &str) -> std::io::Result<()> {
    stream.write_all(&encode_setup_failure(reason))?;
    stream.flush()
}

fn handle_client(stream: &mut UnixStream, shared: &Shared) -> std::io::Result<()> {
    println!("Xylum: client connected");

    let setup = SetupRequest::read_from(stream)?;
    println!(
        "Xylum: setup request: version {}.{}, auth_name: {:?}",
        setup.protocol_major,
        setup.protocol_minor,
        String::from_utf8_lossy(&setup.auth_name)
    );

    if !setup.is_byte_order_ok() {
        return send_setup_failure(stream, "Unsupported byte order");
    }

    if !shared.auth_disabled {
        if setup.auth_name != AUTH_NAME {
            eprintln!(
                "Xylum: auth failed: unsupported auth name {:?}",
                setup.auth_name
            );
            return send_setup_failure(stream, "Unsupported auth name");
        }
        if !validate_cookie(
            &shared.auth_entries,
            DISPLAY_NUMBER,
            &setup.auth_name,
            &setup.auth_data,
        ) {
            eprintln!("Xylum: auth failed: invalid cookie");
            return send_setup_failure(stream, "Authentication failed");
        }
        println!("Xylum: auth ok");
    }

    let resource_id_base = lock(shared).next_client_base();

    let success = SetupSuccess {
        release_number: 1,
        resource_id_base,
        resource_id_mask: RESOURCE_ID_MASK,
        motion_buffer_size: 256,
        max_request_length: 65535,
        image_byte_order: 0,
        bitmap_format_bit_order: 0,
        bitmap_format_scanline_unit: 32,
        bitmap_format_scanline_pad: 32,
        min_keycode: 8,
        max_keycode: 255,
        root_window: ROOT_WINDOW,
        root_visual: ROOT_VISUAL,
        default_colormap: DEFAULT_COLORMAP,
        screen_width: SCREEN_WIDTH,
        screen_height: SCREEN_HEIGHT,
    };

    let result = stream
        .write_all(&success.encode())
        .and_then(|_| stream.flush())
        .and_then(|_| stream.try_clone())
        .and_then(|writer| {
            let conn = Arc::new(ClientConn {
                writer: Mutex::new(writer),
                sequence: AtomicU16::new(0),
            });
            lock(shared).register_client(resource_id_base, Arc::clone(&conn));
            println!("Xylum: setup success sent (resource base {resource_id_base:#x})");
            serve_requests(stream, shared, &conn)
        });

    lock(shared).drop_client(resource_id_base);
    result
}

fn serve_requests(
    stream: &mut UnixStream,
    shared: &Shared,
    conn: &ClientConn,
) -> std::io::Result<()> {
    let mut sequence: u16 = 0;
    loop {
        let raw = match RawRequest::read_from(stream) {
            Ok(Some(raw)) => raw,
            Ok(None) => {
                println!("Xylum: client disconnected");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Xylum: read error: {e}");
                return Ok(());
            }
        };

        sequence = sequence.wrapping_add(1);
        conn.sequence.store(sequence, Ordering::Relaxed);
        println!(
            "Xylum: seq={sequence} {} ({})",
            request_name(raw.opcode),
            raw.opcode
        );

        let (packets, pending) = {
            let mut state = lock(shared);
            let packets = dispatch::process(&mut state, sequence, &raw);
            let pending = std::mem::take(&mut state.pending);
            (packets, pending)
        };

        if !packets.is_empty() {
            let mut writer = conn.writer.lock().unwrap_or_else(|e| e.into_inner());
            for packet in &packets {
                writer.write_all(packet)?;
            }
            writer.flush()?;
        }
        input::send(pending);
    }
}