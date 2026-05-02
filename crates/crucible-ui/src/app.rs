use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Rgba, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px, rgb, rgba,
};
use gpui_component::{
    ActiveTheme as _, Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    dock::{
        DockArea, DockAreaState, DockEvent, DockItem, Panel, PanelControl, PanelEvent, PanelView,
        register_panel,
    },
    input::{Input, InputEvent, InputState},
    scroll::ScrollableElement as _,
};
use thiserror::Error;

use crate::{AssetIndex, AssetItem, AssetKind, SceneModel, ScriptDocument, script::script_files};

const LAYOUT_VERSION: usize = 2;
const LAYOUT_FILE: &str = ".crucible/editor-layout.ron";

#[derive(Debug, Error)]
pub enum UiError {
    #[error("failed to prepare project directories: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to index assets: {0}")]
    Assets(#[from] crate::assets::AssetError),
    #[error("failed to load script: {0}")]
    Script(#[from] crate::script::ScriptError),
    #[error("failed to encode dock layout: {0}")]
    LayoutEncode(#[from] ron::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum MenuId {
    File,
    Edit,
    View,
    Assets,
    Build,
    Run,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
enum CommandId {
    SaveLayout,
    SaveScript,
    RefreshAssets,
    Play,
    Pause,
    Stop,
}

pub struct EditorModel {
    project_root: PathBuf,
    layout_path: PathBuf,
    pub scene: SceneModel,
    pub assets: AssetIndex,
    pub script: ScriptDocument,
    play_mode: bool,
    status: String,
}

impl EditorModel {
    pub fn open(project_root: impl Into<PathBuf>) -> Result<Self, UiError> {
        let project_root = project_root.into();
        let assets_dir = project_root.join("assets");
        let scripts_dir = project_root.join("scripts");

        fs::create_dir_all(&assets_dir)?;
        fs::create_dir_all(&scripts_dir)?;
        ensure_default_script(&scripts_dir)?;

        let assets = AssetIndex::scan(&assets_dir)?;
        let script_path = script_files(&scripts_dir)
            .into_iter()
            .next()
            .unwrap_or_else(|| scripts_dir.join("main.rs"));
        let script = ScriptDocument::load(script_path)?;

        Ok(Self {
            layout_path: project_root.join(LAYOUT_FILE),
            project_root,
            scene: SceneModel::default(),
            assets,
            script,
            play_mode: false,
            status: "Ready".to_string(),
        })
    }

    pub fn layout_path(&self) -> &Path {
        &self.layout_path
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn play_mode(&self) -> bool {
        self.play_mode
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    fn execute(&mut self, command: CommandId) {
        match command {
            CommandId::SaveLayout => {}
            CommandId::SaveScript => match self.script.save() {
                Ok(()) => self.status = format!("Saved {}", self.script.file_name()),
                Err(error) => self.status = format!("Save failed: {error}"),
            },
            CommandId::RefreshAssets => match self.assets.refresh() {
                Ok(()) => self.status = "Assets refreshed".to_string(),
                Err(error) => self.status = format!("Asset refresh failed: {error}"),
            },
            CommandId::Play => {
                self.play_mode = true;
                self.status = "Play mode".to_string();
            }
            CommandId::Pause => {
                self.play_mode = false;
                self.status = "Paused".to_string();
            }
            CommandId::Stop => {
                self.play_mode = false;
                self.status = "Stopped".to_string();
            }
        }
    }
}

pub fn init(cx: &mut App) {
    gpui_component::init(cx);
}

pub struct EditorRoot {
    model: Entity<EditorModel>,
    dock_area: Entity<DockArea>,
    open_menu: Option<MenuId>,
    _subscriptions: Vec<Subscription>,
}

impl EditorRoot {
    pub fn new(
        project_root: impl Into<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let project_root = project_root.into();
        let model = cx.new(|_| {
            EditorModel::open(project_root.clone()).expect("failed to initialize editor model")
        });

        register_editor_panels(cx, model.clone());

        let dock_area =
            cx.new(|cx| DockArea::new("crucible-editor", Some(LAYOUT_VERSION), window, cx));
        let layout_path = model.read(cx).layout_path().to_path_buf();
        let loaded = try_load_dock_layout(&dock_area, &layout_path, window, cx);
        if !loaded {
            apply_default_dock_layout(&dock_area, model.clone(), window, cx);
        }

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&model, |this, _, cx| {
            this.open_menu = None;
            cx.notify();
        }));
        subscriptions.push(cx.subscribe_in(
            &dock_area,
            window,
            move |_, dock_area, event, _, cx| {
                if matches!(event, DockEvent::LayoutChanged) {
                    let state = dock_area.read(cx).dump(cx);
                    let _ = persist_dock_layout(&layout_path, &state);
                }
            },
        ));

        Self {
            model,
            dock_area,
            open_menu: None,
            _subscriptions: subscriptions,
        }
    }

    fn execute(&mut self, command: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        match command {
            CommandId::SaveLayout => {
                let state = self.dock_area.read(cx).dump(cx);
                let result = persist_dock_layout(self.model.read(cx).layout_path(), &state);
                self.model.update(cx, |model, cx| {
                    model.status = match result {
                        Ok(()) => "Layout saved".to_string(),
                        Err(error) => format!("Layout save failed: {error}"),
                    };
                    cx.notify();
                });
            }
            _ => {
                self.model.update(cx, |model, cx| {
                    model.execute(command);
                    cx.notify();
                });
            }
        }
        self.open_menu = None;
        window.refresh();
        cx.notify();
    }

    fn menu_button(
        &self,
        menu: MenuId,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.open_menu == Some(menu);
        Button::new(("menu", menu as usize))
            .label(label)
            .xsmall()
            .ghost()
            .selected(selected)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_menu = if this.open_menu == Some(menu) {
                    None
                } else {
                    Some(menu)
                };
                cx.notify();
            }))
    }

    fn command_button(
        &self,
        command: CommandId,
        label: &'static str,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Button::new(("cmd", command as usize))
            .label(label)
            .xsmall()
            .compact()
            .map(|button| {
                if active {
                    button.primary()
                } else {
                    button.ghost()
                }
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.execute(command, window, cx);
            }))
    }

    fn render_dropdown(
        &self,
        menu: MenuId,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let items: Vec<(&'static str, Option<CommandId>)> = match menu {
            MenuId::File => vec![
                ("Save Script", Some(CommandId::SaveScript)),
                ("Save Layout", Some(CommandId::SaveLayout)),
            ],
            MenuId::Assets => vec![("Refresh Assets", Some(CommandId::RefreshAssets))],
            MenuId::Run => vec![
                ("Play", Some(CommandId::Play)),
                ("Pause", Some(CommandId::Pause)),
                ("Stop", Some(CommandId::Stop)),
            ],
            MenuId::Edit => vec![("Undo", None), ("Redo", None)],
            MenuId::View => vec![("Reset Dock Layout", Some(CommandId::SaveLayout))],
            MenuId::Build => vec![("Build Game", None)],
            MenuId::Help => vec![("Crucible Docs", None)],
        };

        let left = match menu {
            MenuId::File => 88.0,
            MenuId::Edit => 138.0,
            MenuId::View => 188.0,
            MenuId::Assets => 244.0,
            MenuId::Build => 318.0,
            MenuId::Run => 378.0,
            MenuId::Help => 428.0,
        };

        div()
            .absolute()
            .top(px(36.0))
            .left(px(left))
            .w(px(190.0))
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(border())
            .bg(panel_alt())
            .shadow_lg()
            .children(items.into_iter().enumerate().map(|(ix, (label, command))| {
                let disabled = command.is_none();
                div()
                    .id(("menu-item", ix))
                    .h(px(28.0))
                    .px_2()
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .text_size(px(12.0))
                    .text_color(if disabled { muted() } else { text() })
                    .when(!disabled, |row| {
                        row.cursor_pointer()
                            .hover(|row| row.bg(rgb(0x243042)))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(command) = command {
                                    this.execute(command, window, cx);
                                }
                            }))
                    })
                    .child(label)
            }))
    }
}

impl Render for EditorRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let play_mode = self.model.read(cx).play_mode();
        let status = self.model.read(cx).status().to_string();

        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(background())
            .text_color(text())
            .child(
                div()
                    .h(px(36.0))
                    .w_full()
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border())
                    .bg(panel_header())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .mr_3()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_size(px(13.0))
                                    .child("Crucible"),
                            )
                            .child(self.menu_button(MenuId::File, "File", cx))
                            .child(self.menu_button(MenuId::Edit, "Edit", cx))
                            .child(self.menu_button(MenuId::View, "View", cx))
                            .child(self.menu_button(MenuId::Assets, "Assets", cx))
                            .child(self.menu_button(MenuId::Build, "Build", cx))
                            .child(self.menu_button(MenuId::Run, "Run", cx))
                            .child(self.menu_button(MenuId::Help, "Help", cx)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().mr_2().text_xs().text_color(muted()).child(status))
                            .child(self.command_button(CommandId::SaveScript, "Save", false, cx))
                            .child(self.command_button(CommandId::Play, "Play", play_mode, cx))
                            .child(self.command_button(CommandId::Pause, "Pause", false, cx))
                            .child(self.command_button(CommandId::Stop, "Stop", false, cx)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(36.0))
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .child(self.dock_area.clone()),
            )
            .when_some(self.open_menu, |this, menu| {
                this.child(self.render_dropdown(menu, window, cx))
            })
    }
}

struct ViewportPanel {
    model: Entity<EditorModel>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl ViewportPanel {
    const NAME: &'static str = "crucible.viewport";

    fn new(model: Entity<EditorModel>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            focus: cx.focus_handle(),
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for ViewportPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let play_mode = self.model.read(cx).play_mode();
        let grid = rgba(0x25303d80);
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x090c10))
            .child(
                div()
                    .absolute()
                    .size_full()
                    .children((1..18).map(move |ix| {
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(ix as f32 * 48.0))
                            .w(px(1.0))
                            .bg(grid)
                    }))
                    .children((1..12).map(move |ix| {
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(px(ix as f32 * 48.0))
                            .h(px(1.0))
                            .bg(grid)
                    })),
            )
            .child(
                div()
                    .absolute()
                    .top(px(14.0))
                    .left(px(14.0))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(rgba(0x121821db))
                    .border_1()
                    .border_color(border())
                    .text_xs()
                    .text_color(muted())
                    .child(if play_mode {
                        "Viewport - running"
                    } else {
                        "Viewport - edit mode"
                    }),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(14.0))
                    .right(px(14.0))
                    .text_xs()
                    .text_color(muted())
                    .child("3D render target placeholder"),
            )
    }
}

struct SceneOutlinePanel {
    model: Entity<EditorModel>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SceneOutlinePanel {
    const NAME: &'static str = "crucible.scene-outline";

    fn new(model: Entity<EditorModel>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            focus: cx.focus_handle(),
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for SceneOutlinePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.model.read(cx).scene.visible_rows();
        let selected = self.model.read(cx).scene.selected();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel())
            .child(section_header("Scene"))
            .child(
                div()
                    .id("scene-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_1()
                    .children(rows.into_iter().map(|row| {
                        let model = self.model.clone();
                        let model_for_toggle = self.model.clone();
                        let is_selected = selected == Some(row.id);
                        let marker = if row.has_children {
                            if row.expanded { "v" } else { ">" }
                        } else {
                            ""
                        };

                        div()
                            .id(("scene-row", row.id.0))
                            .h(px(26.0))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_pointer()
                            .bg(if is_selected {
                                accent_soft()
                            } else {
                                transparent()
                            })
                            .hover(|row| row.bg(rgb(0x202936)))
                            .on_click(move |_, _, cx| {
                                model.update(cx, |model, cx| {
                                    model.scene.select(row.id);
                                    cx.notify();
                                });
                            })
                            .child(
                                div()
                                    .w(px(12.0 + row.depth as f32 * 14.0))
                                    .flex()
                                    .justify_end()
                                    .text_color(muted())
                                    .id(("scene-toggle", row.id.0))
                                    .child(marker)
                                    .when(row.has_children, |toggle| {
                                        toggle.on_click(move |_, _, cx| {
                                            cx.stop_propagation();
                                            model_for_toggle.update(cx, |model, cx| {
                                                model.scene.toggle_expanded(row.id);
                                                cx.notify();
                                            });
                                        })
                                    }),
                            )
                            .child(
                                div()
                                    .overflow_x_hidden()
                                    .text_size(px(12.0))
                                    .text_color(text())
                                    .child(row.name),
                            )
                    })),
            )
    }
}

struct InspectorPanel {
    model: Entity<EditorModel>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl InspectorPanel {
    const NAME: &'static str = "crucible.inspector";

    fn new(model: Entity<EditorModel>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            focus: cx.focus_handle(),
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for InspectorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.read(cx);
        let name = model.scene.selected_name().unwrap_or("None").to_string();
        let selected = model.scene.selected().map(|id| id.0).unwrap_or_default();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel())
            .child(section_header("Inspector"))
            .child(
                div()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(property_row("Name", name))
                    .child(property_row("Node Id", selected.to_string()))
                    .child(property_row("Transform", "0, 0, 0"))
                    .child(property_row("Components", "Transform, Mesh"))
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(muted())
                            .child("Component editing and script bindings attach here."),
                    ),
            )
    }
}

struct AssetManagerPanel {
    model: Entity<EditorModel>,
    filter_input: Entity<InputState>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl AssetManagerPanel {
    const NAME: &'static str = "crucible.asset-manager";

    fn new(model: Entity<EditorModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter = model.read(cx).assets.filter().to_string();
        let filter_input = cx.new(|cx| {
            let mut input = InputState::new(window, cx).placeholder("Filter assets");
            input.set_value(filter, window, cx);
            input
        });

        let mut subscriptions = vec![cx.observe(&model, |_, _, cx| cx.notify())];
        subscriptions.push(cx.subscribe(&filter_input, {
            let model = model.clone();
            move |_, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let filter = input.read(cx).text().to_string();
                    model.update(cx, |model, cx| {
                        model.assets.set_filter(filter);
                        cx.notify();
                    });
                }
            }
        }));

        Self {
            model,
            filter_input,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }
}

impl Render for AssetManagerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let items: Vec<AssetItem> = self
            .model
            .read(cx)
            .assets
            .visible_items()
            .into_iter()
            .cloned()
            .collect();
        let selected = self.model.read(cx).assets.selected().map(Path::to_path_buf);
        let status = self.model.read(cx).status().to_string();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(panel())
            .child(
                div()
                    .h(px(42.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_b_1()
                    .border_color(border())
                    .child(
                        div()
                            .w(px(260.0))
                            .child(Input::new(&self.filter_input).small()),
                    )
                    .child(div().text_xs().text_color(muted()).child(status)),
            )
            .child(
                div()
                    .id("asset-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .p_2()
                    .children(items.into_iter().enumerate().map(|(ix, item)| {
                        let model = self.model.clone();
                        let path = item.path.clone();
                        let is_selected = selected.as_ref() == Some(&item.path);

                        div()
                            .id(("asset-row", ix))
                            .h(px(26.0))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .gap_2()
                            .cursor_pointer()
                            .bg(if is_selected {
                                accent_soft()
                            } else {
                                transparent()
                            })
                            .hover(|row| row.bg(rgb(0x202936)))
                            .on_click(move |_, _, cx| {
                                model.update(cx, |model, cx| {
                                    model.assets.select(path.clone());
                                    model.status = format!("Selected {}", path.display());
                                    cx.notify();
                                });
                            })
                            .child(
                                div()
                                    .w(px(40.0))
                                    .text_xs()
                                    .text_color(accent())
                                    .child(asset_kind_label(item.kind)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .overflow_x_hidden()
                                    .text_size(px(12.0))
                                    .text_color(text())
                                    .child(item.display_path),
                            )
                    })),
            )
    }
}

struct ScriptEditorPanel {
    model: Entity<EditorModel>,
    input: Entity<InputState>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl ScriptEditorPanel {
    const NAME: &'static str = "crucible.script-editor";

    fn new(model: Entity<EditorModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = model.read(cx).script.buffer.text().to_string();
        let input = cx.new(|cx| {
            let mut input = InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .searchable(true);
            input.set_value(text, window, cx);
            input
        });

        let mut subscriptions = vec![cx.observe(&model, |_, _, cx| cx.notify())];
        subscriptions.push(cx.subscribe(&input, {
            let model = model.clone();
            move |_, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = input.read(cx).text().to_string();
                    model.update(cx, |model, cx| {
                        model.script.set_text(text);
                        model.status = "Script modified".to_string();
                        cx.notify();
                    });
                }
            }
        }));

        Self {
            model,
            input,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }
}

impl Render for ScriptEditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file_name = self.model.read(cx).script.file_name();
        let model = self.model.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0c0f14))
            .child(
                div()
                    .h(px(34.0))
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(border())
                    .bg(panel_header())
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_sm()
                            .bg(panel_alt())
                            .text_xs()
                            .font_family(cx.theme().mono_font_family.clone())
                            .child(file_name),
                    )
                    .child(
                        Button::new("script-save")
                            .label("Save")
                            .xsmall()
                            .ghost()
                            .on_click(move |_, _, cx| {
                                model.update(cx, |model, cx| {
                                    model.execute(CommandId::SaveScript);
                                    cx.notify();
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .child(Input::new(&self.input).h_full().appearance(false)),
            )
    }
}

macro_rules! impl_panel {
    ($ty:ty, $title:literal) => {
        impl EventEmitter<PanelEvent> for $ty {}

        impl Focusable for $ty {
            fn focus_handle(&self, _cx: &App) -> FocusHandle {
                self.focus.clone()
            }
        }

        impl Panel for $ty {
            fn panel_name(&self) -> &'static str {
                Self::NAME
            }

            fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
                SharedString::from($title)
            }

            fn tab_name(&self, _cx: &App) -> Option<SharedString> {
                Some(SharedString::from($title))
            }

            fn closable(&self, _cx: &App) -> bool {
                false
            }

            fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
                Some(PanelControl::Both)
            }

            fn inner_padding(&self, _cx: &App) -> bool {
                false
            }
        }
    };
}

impl_panel!(ViewportPanel, "Viewport");
impl_panel!(SceneOutlinePanel, "Scene Outline");
impl_panel!(InspectorPanel, "Inspector");
impl_panel!(AssetManagerPanel, "Asset Manager");
impl_panel!(ScriptEditorPanel, "Script Editor");

fn register_editor_panels(cx: &mut App, model: Entity<EditorModel>) {
    register_panel(cx, ViewportPanel::NAME, {
        let model = model.clone();
        move |_, _, _, window, cx| {
            Box::new(cx.new(|cx| ViewportPanel::new(model.clone(), window, cx)))
                as Box<dyn PanelView>
        }
    });
    register_panel(cx, SceneOutlinePanel::NAME, {
        let model = model.clone();
        move |_, _, _, window, cx| {
            Box::new(cx.new(|cx| SceneOutlinePanel::new(model.clone(), window, cx)))
                as Box<dyn PanelView>
        }
    });
    register_panel(cx, InspectorPanel::NAME, {
        let model = model.clone();
        move |_, _, _, window, cx| {
            Box::new(cx.new(|cx| InspectorPanel::new(model.clone(), window, cx)))
                as Box<dyn PanelView>
        }
    });
    register_panel(cx, AssetManagerPanel::NAME, {
        let model = model.clone();
        move |_, _, _, window, cx| {
            Box::new(cx.new(|cx| AssetManagerPanel::new(model.clone(), window, cx)))
                as Box<dyn PanelView>
        }
    });
    register_panel(cx, ScriptEditorPanel::NAME, move |_, _, _, window, cx| {
        Box::new(cx.new(|cx| ScriptEditorPanel::new(model.clone(), window, cx)))
            as Box<dyn PanelView>
    });
}

fn apply_default_dock_layout(
    dock_area: &Entity<DockArea>,
    model: Entity<EditorModel>,
    window: &mut Window,
    cx: &mut App,
) {
    let viewport = cx.new(|cx| ViewportPanel::new(model.clone(), window, cx));
    let scene = cx.new(|cx| SceneOutlinePanel::new(model.clone(), window, cx));
    let inspector = cx.new(|cx| InspectorPanel::new(model.clone(), window, cx));
    let assets = cx.new(|cx| AssetManagerPanel::new(model.clone(), window, cx));
    let scripts = cx.new(|cx| ScriptEditorPanel::new(model, window, cx));

    let weak = dock_area.downgrade();
    let center = DockItem::tabs(vec![Arc::new(viewport)], &weak, window, cx);
    let left = DockItem::tabs(vec![Arc::new(scene)], &weak, window, cx);
    let right = DockItem::tabs(vec![Arc::new(inspector)], &weak, window, cx);
    let bottom = DockItem::tabs(vec![Arc::new(assets), Arc::new(scripts)], &weak, window, cx);

    dock_area.update(cx, |dock, cx| {
        dock.set_center(center, window, cx);
        dock.set_left_dock(left, Some(px(260.0)), true, window, cx);
        dock.set_right_dock(right, Some(px(300.0)), true, window, cx);
        dock.set_bottom_dock(bottom, Some(px(260.0)), true, window, cx);
    });
}

fn try_load_dock_layout(
    dock_area: &Entity<DockArea>,
    path: &Path,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(state) = ron::from_str::<DockAreaState>(&raw) else {
        return false;
    };
    if state.version != Some(LAYOUT_VERSION) {
        return false;
    }

    dock_area.update(cx, |dock, cx| dock.load(state, window, cx).is_ok())
}

fn persist_dock_layout(path: &Path, state: &DockAreaState) -> Result<(), UiError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let pretty = ron::ser::PrettyConfig::default();
    fs::write(path, ron::ser::to_string_pretty(state, pretty)?)?;
    Ok(())
}

fn ensure_default_script(scripts_dir: &Path) -> Result<(), std::io::Error> {
    let main = scripts_dir.join("main.rs");
    if main.exists() {
        return Ok(());
    }

    fs::write(
        main,
        "pub struct PlayerController {\n    pub speed: f32,\n}\n\nimpl PlayerController {\n    pub fn update(&mut self, delta_seconds: f32) {\n        let _movement = self.speed * delta_seconds;\n    }\n}\n",
    )
}

fn section_header(label: &'static str) -> impl IntoElement {
    div()
        .h(px(32.0))
        .px_3()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(border())
        .bg(panel_header())
        .text_xs()
        .text_color(muted())
        .child(label)
}

fn property_row(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .min_h(px(30.0))
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .border_color(border())
        .bg(panel_alt())
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(div().text_xs().text_color(muted()).child(label))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(text())
                .overflow_x_hidden()
                .child(value.into()),
        )
}

fn asset_kind_label(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Folder => "DIR",
        AssetKind::Mesh => "3D",
        AssetKind::Texture => "IMG",
        AssetKind::Script => "RS",
        AssetKind::Material => "MAT",
        AssetKind::Audio => "AUD",
        AssetKind::Other => "FILE",
    }
}

fn transparent() -> Rgba {
    rgba(0x00000000)
}

fn background() -> Rgba {
    rgb(0x0d1015)
}

fn panel() -> Rgba {
    rgb(0x11151b)
}

fn panel_alt() -> Rgba {
    rgb(0x171c23)
}

fn panel_header() -> Rgba {
    rgb(0x141922)
}

fn border() -> Rgba {
    rgb(0x2a323d)
}

fn text() -> Rgba {
    rgb(0xe5e9ef)
}

fn muted() -> Rgba {
    rgb(0x8b96a7)
}

fn accent() -> Rgba {
    rgb(0x66d9c7)
}

fn accent_soft() -> Rgba {
    rgb(0x173c3a)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::SceneNodeId;

    #[test]
    fn editor_model_creates_project_directories() {
        let root = std::env::temp_dir().join(format!(
            "crucible-gpui-model-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let model = EditorModel::open(&root).unwrap();

        assert!(model.project_root().join("assets").exists());
        assert!(
            model
                .project_root()
                .join("scripts")
                .join("main.rs")
                .exists()
        );
        assert_eq!(model.scene.selected(), Some(SceneNodeId(1)));

        fs::remove_dir_all(root).unwrap();
    }
}
