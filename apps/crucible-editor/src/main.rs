use std::borrow::Cow;

use anyhow::Result;
use gpui::{
    App, AppContext as _, Application, AssetSource, Bounds, SharedString, WindowBounds,
    WindowDecorations, WindowOptions, px, size,
};
use gpui_component::{Root, TitleBar};
use tracing_subscriber::EnvFilter;

use crucible_ui::EditorRoot;

struct CrucibleAssets;

impl AssetSource for CrucibleAssets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let svg = match path {
            "icons/check.svg" => Some(CHECK_SVG),
            "icons/chevron-down.svg" => Some(CHEVRON_DOWN_SVG),
            "icons/chevron-left.svg" => Some(CHEVRON_LEFT_SVG),
            "icons/chevron-right.svg" => Some(CHEVRON_RIGHT_SVG),
            "icons/chevron-up.svg" => Some(CHEVRON_UP_SVG),
            "icons/close.svg" => Some(CLOSE_SVG),
            "icons/ellipsis.svg" => Some(ELLIPSIS_SVG),
            "icons/external-link.svg" => Some(EXTERNAL_LINK_SVG),
            "icons/maximize.svg" => Some(MAXIMIZE_SVG),
            "icons/minimize.svg" => Some(MINIMIZE_SVG),
            "icons/panel-bottom.svg" => Some(PANEL_BOTTOM_SVG),
            "icons/panel-bottom-open.svg" => Some(PANEL_BOTTOM_OPEN_SVG),
            "icons/panel-left.svg" => Some(PANEL_LEFT_SVG),
            "icons/panel-left-open.svg" => Some(PANEL_LEFT_OPEN_SVG),
            "icons/panel-right.svg" => Some(PANEL_RIGHT_SVG),
            "icons/panel-right-open.svg" => Some(PANEL_RIGHT_OPEN_SVG),
            "icons/resize-corner.svg" => Some(RESIZE_CORNER_SVG),
            "icons/search.svg" => Some(SEARCH_SVG),
            _ => None,
        };

        Ok(svg.map(|svg| Cow::Borrowed(svg.as_bytes())))
    }

    fn list(&self, _path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

const CHECK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4 10-10"/></svg>"#;
const CHEVRON_DOWN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
const CHEVRON_LEFT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"><path d="m15 6-6 6 6 6"/></svg>"#;
const CHEVRON_RIGHT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"><path d="m9 6 6 6-6 6"/></svg>"#;
const CHEVRON_UP_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round" stroke-linejoin="round"><path d="m6 15 6-6 6 6"/></svg>"#;
const CLOSE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round"><path d="M6 6l12 12"/><path d="M18 6 6 18"/></svg>"#;
const ELLIPSIS_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor"><circle cx="5" cy="12" r="1.8"/><circle cx="12" cy="12" r="1.8"/><circle cx="19" cy="12" r="1.8"/></svg>"#;
const EXTERNAL_LINK_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 5h5v5"/><path d="m19 5-9 9"/><path d="M12 5H6a1 1 0 0 0-1 1v12a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-6"/></svg>"#;
const MAXIMIZE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3"/><path d="M16 3h3a2 2 0 0 1 2 2v3"/><path d="M21 16v3a2 2 0 0 1-2 2h-3"/><path d="M8 21H5a2 2 0 0 1-2-2v-3"/></svg>"#;
const MINIMIZE_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3v3a2 2 0 0 1-2 2H3"/><path d="M16 3v3a2 2 0 0 0 2 2h3"/><path d="M21 16h-3a2 2 0 0 0-2 2v3"/><path d="M3 16h3a2 2 0 0 1 2 2v3"/></svg>"#;
const PANEL_BOTTOM_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 15h18"/></svg>"#;
const PANEL_BOTTOM_OPEN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 15h18"/><path d="m9 10 3 3 3-3"/></svg>"#;
const PANEL_LEFT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/></svg>"#;
const PANEL_LEFT_OPEN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M9 4v16"/><path d="m15 9-3 3 3 3"/></svg>"#;
const PANEL_RIGHT_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/></svg>"#;
const PANEL_RIGHT_OPEN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="16" rx="2"/><path d="M15 4v16"/><path d="m9 9 3 3-3 3"/></svg>"#;
const RESIZE_CORNER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21 14v7h-7"/><path d="M21 9 9 21"/><path d="M16 9 9 16"/></svg>"#;
const SEARCH_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.25" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg>"#;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crucible_editor=info,crucible_ui=info".into()),
        )
        .init();

    let project_root = std::env::current_dir()?;
    let app = Application::new().with_assets(CrucibleAssets);

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
