use std::time::Instant;

use anyhow::{Context, Result};
use crucible_core::{Engine, EngineConfig};
use crucible_render::{ClearColor, GpuRenderer, RenderError, RenderOptions};
use crucible_ui::{EditorState, KeyModifiers, Point, PointerButton, Size, UiKey, UiRenderer};
use tracing::{error, warn};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
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
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = EditorApp::default();
    event_loop
        .run_app(&mut app)
        .context("editor event loop failed")
}

struct EditorApp {
    engine: Engine,
    window: Option<&'static Window>,
    renderer: Option<GpuRenderer<'static>>,
    ui_renderer: Option<UiRenderer>,
    editor: Option<EditorState>,
    modifiers: KeyModifiers,
    last_pointer: Point,
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
            ui_renderer: None,
            editor: None,
            modifiers: KeyModifiers::default(),
            last_pointer: Point::new(0.0, 0.0),
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
        window.set_ime_allowed(true);

        self.engine
            .initialize()
            .expect("engine failed to initialize");
        let renderer = pollster::block_on(GpuRenderer::new(window, RenderOptions::default()))
            .expect("renderer failed to initialize");
        let ui_renderer = UiRenderer::new(
            renderer.device(),
            renderer.queue(),
            renderer.surface_format(),
        );
        let editor = EditorState::open(std::env::current_dir().expect("failed to read cwd"))
            .expect("failed to initialize editor state");

        self.renderer = Some(renderer);
        self.ui_renderer = Some(ui_renderer);
        self.editor = Some(editor);
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
                if let Some(editor) = self.editor.as_mut() {
                    editor.invalidate();
                }
                window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.last_pointer = point_from_position(position);
                if let Some(editor) = self.editor.as_mut() {
                    editor.handle_pointer_move(self.last_pointer);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(editor) = self.editor.as_mut() {
                    if state == ElementState::Pressed {
                        editor.handle_pointer_down(self.last_pointer, pointer_button(button));
                    } else {
                        editor.handle_pointer_up(self.last_pointer, pointer_button(button));
                    }
                }
                window.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.handle_scroll(scroll_delta_y(delta));
                }
                window.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.modifiers = KeyModifiers {
                    ctrl: state.control_key(),
                    shift: state.shift_key(),
                    alt: state.alt_key(),
                };
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let Some(editor) = self.editor.as_mut() {
                        if let Some(key) = map_key(&event.logical_key, self.modifiers) {
                            editor.handle_key(key, self.modifiers);
                        } else if !self.modifiers.ctrl
                            && !self.modifiers.alt
                            && let Some(text) = &event.text
                        {
                            editor.handle_text_input(text.as_str());
                        }
                    }
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.draw_frame();
                if self
                    .editor
                    .as_ref()
                    .is_some_and(EditorState::needs_continuous_repaint)
                {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(editor)) = (self.window, self.editor.as_mut()) {
            if editor.take_repaint_request() || editor.needs_continuous_repaint() {
                window.request_redraw();
            }
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
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        let Some(ui_renderer) = self.ui_renderer.as_mut() else {
            return;
        };

        match renderer.begin_frame(clear_color) {
            Ok(mut frame) => {
                let (width, height) = renderer.size();
                let draw_list = editor.draw(Size::new(width as f32, height as f32));
                let (encoder, view) = frame.encoder_and_view_mut();
                if let Err(error) = ui_renderer.render(
                    renderer.device(),
                    renderer.queue(),
                    encoder,
                    view,
                    Size::new(width as f32, height as f32),
                    &draw_list,
                ) {
                    error!(%error, "UI rendering failed");
                }
                renderer.submit_frame(frame);
            }
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

fn point_from_position(position: PhysicalPosition<f64>) -> Point {
    Point::new(position.x as f32, position.y as f32)
}

fn pointer_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        _ => PointerButton::Primary,
    }
}

fn scroll_delta_y(delta: MouseScrollDelta) -> f32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => y * 38.0,
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    }
}

fn map_key(key: &Key, modifiers: KeyModifiers) -> Option<UiKey> {
    if modifiers.ctrl
        && let Key::Character(text) = key
        && text.eq_ignore_ascii_case("s")
    {
        return Some(UiKey::Save);
    }

    match key {
        Key::Named(NamedKey::Backspace) => Some(UiKey::Backspace),
        Key::Named(NamedKey::Delete) => Some(UiKey::Delete),
        Key::Named(NamedKey::Enter) => Some(UiKey::Enter),
        Key::Named(NamedKey::Escape) => Some(UiKey::Escape),
        Key::Named(NamedKey::ArrowLeft) => Some(UiKey::ArrowLeft),
        Key::Named(NamedKey::ArrowRight) => Some(UiKey::ArrowRight),
        Key::Named(NamedKey::ArrowUp) => Some(UiKey::ArrowUp),
        Key::Named(NamedKey::ArrowDown) => Some(UiKey::ArrowDown),
        _ => None,
    }
}
