use std::time::Instant;

use anyhow::{Context, Result};
use crucible_core::{Engine, EngineConfig};
use crucible_render::{ClearColor, GpuRenderer, RenderError, RenderOptions};
use tracing::{error, warn};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crucible_editor=info,crucible_render=info".into()),
        )
        .init();

    let event_loop = EventLoop::new().context("failed to create editor event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = EditorApp::default();
    event_loop
        .run_app(&mut app)
        .context("editor event loop failed")
}

struct EditorApp {
    engine: Engine,
    window: Option<&'static Window>,
    renderer: Option<GpuRenderer<'static>>,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self {
            engine: Engine::new(EngineConfig {
                app_name: "Crucible Editor".to_string(),
                ..EngineConfig::default()
            }),
            window: None,
            renderer: None,
        }
    }
}

impl ApplicationHandler for EditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title(self.engine.config().app_name.clone())
            .with_inner_size(LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(LogicalSize::new(960.0, 540.0));

        let window = event_loop
            .create_window(attributes)
            .expect("failed to create editor window");
        let window = Box::leak(Box::new(window));

        self.engine
            .initialize()
            .expect("engine failed to initialize");
        self.renderer = Some(
            pollster::block_on(GpuRenderer::new(window, RenderOptions::default()))
                .expect("renderer failed to initialize"),
        );
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                if let Err(error) = self.engine.shutdown() {
                    error!(%error, "engine shutdown failed");
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                self.draw_frame();
                window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window {
            window.request_redraw();
        }
    }
}

impl EditorApp {
    fn draw_frame(&mut self) {
        let frame = self
            .engine
            .tick(Instant::now())
            .expect("engine update failed during frame");
        let pulse = (frame.total_seconds().sin() as f64 + 1.0) * 0.5;
        let clear_color = ClearColor {
            red: ClearColor::CRUCIBLE_DARK.red + 0.02 * pulse,
            green: ClearColor::CRUCIBLE_DARK.green + 0.03 * pulse,
            blue: ClearColor::CRUCIBLE_DARK.blue + 0.05 * pulse,
            alpha: 1.0,
        };

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        match renderer.render(clear_color) {
            Ok(()) => {}
            Err(RenderError::SurfaceLost | RenderError::SurfaceOutdated) => {
                let (width, height) = renderer.size();
                renderer.resize(width, height);
            }
            Err(RenderError::SurfaceTimeout) => {
                warn!("GPU surface timed out while acquiring a frame");
            }
            Err(RenderError::SurfaceOccluded) => {}
            Err(error) => {
                error!(%error, "rendering failed");
            }
        }
    }
}
