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

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::input;
use crate::state::{ServerState, SCREEN_HEIGHT, SCREEN_WIDTH};

struct Presenter {
    state: Arc<Mutex<ServerState>>,
    window: Option<Arc<Window>>,
    context: Option<softbuffer::Context<Arc<Window>>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
}

impl Presenter {
    fn lock(&self) -> MutexGuard<'_, ServerState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn redraw(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(width), Some(height)) = (
            NonZeroU32::new(size.width),
            NonZeroU32::new(size.height),
        ) else {
            return;
        };
        if surface.resize(width, height).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };

        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let stride = width.get() as usize;
            let rows = (height.get() as usize).min(state.framebuffer.height);
            let columns = stride.min(state.framebuffer.width);
            for y in 0..rows {
                let source = y * state.framebuffer.width;
                let target = y * stride;
                buffer[target..target + columns]
                    .copy_from_slice(&state.framebuffer.pixels[source..source + columns]);
            }
            state.dirty = false;
        }

        let _ = buffer.present();
    }
}

impl ApplicationHandler for Presenter {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Xylum")
            .with_inner_size(PhysicalSize::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32))
            .with_resizable(false);
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(e) => {
                eprintln!("Xylum: could not create window: {e}");
                event_loop.exit();
                return;
            }
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(context) => context,
            Err(e) => {
                eprintln!("Xylum: could not create graphics context: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(e) => {
                eprintln!("Xylum: could not create surface: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x.round().clamp(0.0, SCREEN_WIDTH as f64 - 1.0) as i16;
                let y = position.y.round().clamp(0.0, SCREEN_HEIGHT as f64 - 1.0) as i16;
                let deliveries = {
                    let mut state = self.lock();
                    state.pointer = (x, y);
                    input::motion_event(&mut state)
                };
                input::send(deliveries);
            }
            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                let number = match button {
                    MouseButton::Left => 1u8,
                    MouseButton::Middle => 2,
                    MouseButton::Right => 3,
                    _ => return,
                };
                let pressed = button_state == ElementState::Pressed;
                let deliveries = {
                    let mut state = self.lock();
                    input::button_event(&mut state, number, pressed)
                };
                input::send(deliveries);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let vertical = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(position) => position.y,
                };
                if vertical == 0.0 {
                    return;
                }
                let number = if vertical > 0.0 { 4u8 } else { 5u8 };
                let deliveries = {
                    let mut state = self.lock();
                    let mut all = input::button_event(&mut state, number, true);
                    all.extend(input::button_event(&mut state, number, false));
                    all
                };
                input::send(deliveries);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let Some(evdev) = evdev_code(code) else {
                    return;
                };
                let pressed = event.state == ElementState::Pressed;
                let deliveries = {
                    let mut state = self.lock();
                    input::key_event(&mut state, evdev + 8, pressed, event.repeat)
                };
                input::send(deliveries);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let dirty = self.lock().dirty;
        if dirty {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}

fn evdev_code(code: KeyCode) -> Option<u8> {
    let evdev = match code {
        KeyCode::Escape => 1,
        KeyCode::Digit1 => 2,
        KeyCode::Digit2 => 3,
        KeyCode::Digit3 => 4,
        KeyCode::Digit4 => 5,
        KeyCode::Digit5 => 6,
        KeyCode::Digit6 => 7,
        KeyCode::Digit7 => 8,
        KeyCode::Digit8 => 9,
        KeyCode::Digit9 => 10,
        KeyCode::Digit0 => 11,
        KeyCode::Minus => 12,
        KeyCode::Equal => 13,
        KeyCode::Backspace => 14,
        KeyCode::Tab => 15,
        KeyCode::KeyQ => 16,
        KeyCode::KeyW => 17,
        KeyCode::KeyE => 18,
        KeyCode::KeyR => 19,
        KeyCode::KeyT => 20,
        KeyCode::KeyY => 21,
        KeyCode::KeyU => 22,
        KeyCode::KeyI => 23,
        KeyCode::KeyO => 24,
        KeyCode::KeyP => 25,
        KeyCode::BracketLeft => 26,
        KeyCode::BracketRight => 27,
        KeyCode::Enter => 28,
        KeyCode::ControlLeft => 29,
        KeyCode::KeyA => 30,
        KeyCode::KeyS => 31,
        KeyCode::KeyD => 32,
        KeyCode::KeyF => 33,
        KeyCode::KeyG => 34,
        KeyCode::KeyH => 35,
        KeyCode::KeyJ => 36,
        KeyCode::KeyK => 37,
        KeyCode::KeyL => 38,
        KeyCode::Semicolon => 39,
        KeyCode::Quote => 40,
        KeyCode::Backquote => 41,
        KeyCode::ShiftLeft => 42,
        KeyCode::Backslash => 43,
        KeyCode::KeyZ => 44,
        KeyCode::KeyX => 45,
        KeyCode::KeyC => 46,
        KeyCode::KeyV => 47,
        KeyCode::KeyB => 48,
        KeyCode::KeyN => 49,
        KeyCode::KeyM => 50,
        KeyCode::Comma => 51,
        KeyCode::Period => 52,
        KeyCode::Slash => 53,
        KeyCode::ShiftRight => 54,
        KeyCode::AltLeft => 56,
        KeyCode::Space => 57,
        KeyCode::CapsLock => 58,
        KeyCode::F1 => 59,
        KeyCode::F2 => 60,
        KeyCode::F3 => 61,
        KeyCode::F4 => 62,
        KeyCode::F5 => 63,
        KeyCode::F6 => 64,
        KeyCode::F7 => 65,
        KeyCode::F8 => 66,
        KeyCode::F9 => 67,
        KeyCode::F10 => 68,
        KeyCode::NumLock => 69,
        KeyCode::ScrollLock => 70,
        KeyCode::IntlBackslash => 86,
        KeyCode::F11 => 87,
        KeyCode::F12 => 88,
        KeyCode::ControlRight => 97,
        KeyCode::AltRight => 100,
        KeyCode::Home => 102,
        KeyCode::ArrowUp => 103,
        KeyCode::PageUp => 104,
        KeyCode::ArrowLeft => 105,
        KeyCode::ArrowRight => 106,
        KeyCode::End => 107,
        KeyCode::ArrowDown => 108,
        KeyCode::PageDown => 109,
        KeyCode::Insert => 110,
        KeyCode::Delete => 111,
        KeyCode::SuperLeft => 125,
        KeyCode::SuperRight => 126,
        KeyCode::IntlRo => 89,
        KeyCode::IntlYen => 124,
        KeyCode::Numpad0 => 82,
        KeyCode::Numpad1 => 79,
        KeyCode::Numpad2 => 80,
        KeyCode::Numpad3 => 81,
        KeyCode::Numpad4 => 75,
        KeyCode::Numpad5 => 76,
        KeyCode::Numpad6 => 77,
        KeyCode::Numpad7 => 71,
        KeyCode::Numpad8 => 72,
        KeyCode::Numpad9 => 73,
        KeyCode::NumpadDecimal => 83,
        KeyCode::NumpadAdd => 78,
        KeyCode::NumpadSubtract => 74,
        KeyCode::NumpadMultiply => 55,
        KeyCode::NumpadDivide => 98,
        KeyCode::NumpadEnter => 96,
        KeyCode::NumpadEqual => 117,
        KeyCode::NumpadComma => 121,
        KeyCode::PrintScreen => 99,
        KeyCode::Pause => 119,
        KeyCode::ContextMenu => 127,
        _ => return None,
    };
    Some(evdev)
}

pub fn run(state: Arc<Mutex<ServerState>>) {
    let event_loop = match EventLoop::new() {
        Ok(event_loop) => event_loop,
        Err(e) => {
            eprintln!("Xylum: could not create event loop: {e}");
            return;
        }
    };
    let mut presenter = Presenter {
        state,
        window: None,
        context: None,
        surface: None,
    };
    if let Err(e) = event_loop.run_app(&mut presenter) {
        eprintln!("Xylum: presenter error: {e}");
    }
}