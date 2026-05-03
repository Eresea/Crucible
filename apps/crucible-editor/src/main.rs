use std::borrow::Cow;

use anyhow::Result;
use gpui::{
    App, AppContext as _, Application, Bounds, WindowBounds, WindowDecorations, WindowOptions, px,
    size,
};
use gpui_component::{Root, TitleBar};
use tracing_subscriber::EnvFilter;

use crucible_ui::EditorRoot;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crucible_editor=info,crucible_ui=info".into()),
        )
        .init();

    let project_root = std::env::current_dir()?;
    let app = Application::new();

    app.run(move |cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../../../assets/fonts/MaterialSymbolsOutlined-Regular.ttf"
            ))])
            .expect("failed to load Material Symbols font");

        crucible_ui::init(cx);

        let bounds = Bounds::centered(None, size(px(1280.0), px(720.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()),
                window_decorations: Some(WindowDecorations::Client),
                window_min_size: Some(size(px(960.0), px(540.0))),
                ..Default::default()
            },
            {
                let project_root = project_root.clone();
                move |window, cx| {
                    let editor = cx.new(|cx| EditorRoot::new(project_root.clone(), window, cx));
                    cx.new(|cx| Root::new(editor, window, cx))
                }
            },
        )
        .expect("failed to open Crucible Editor window");

        cx.activate(true);
    });

    Ok(())
}
