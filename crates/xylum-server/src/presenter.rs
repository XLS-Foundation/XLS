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
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

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
                self.lock().pointer = (x, y);
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
