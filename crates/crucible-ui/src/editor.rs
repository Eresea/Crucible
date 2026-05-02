use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    AssetIndex, Color, DockLayout, DockRegion, DrawList, HighlightKind, HighlightSpan,
    KeyModifiers, PanelId, Point, PointerButton, Rect, RustHighlighter, SceneModel, SceneNodeId,
    ScriptDocument, Size, Splitter, Theme, UiKey,
    script::{line_start_offsets, script_files},
};

const ROW_HEIGHT: f32 = 22.0;
const TAB_HEIGHT: f32 = 28.0;
const PADDING: f32 = 8.0;

pub trait EditorPanel {
    fn id(&self) -> PanelId;
    fn title(&self) -> &'static str {
        self.id().title()
    }
    fn visible(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    SaveLayout,
    SaveScript,
    RefreshAssets,
    Play,
    Pause,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MenuId {
    File,
    Edit,
    View,
    Assets,
    Build,
    Run,
    Help,
}

#[derive(Debug, Clone)]
struct TabHit {
    panel: PanelId,
    region: DockRegion,
    rect: Rect,
}

#[derive(Debug, Clone)]
struct RegionHit {
    region: DockRegion,
    rect: Rect,
}

#[derive(Debug, Clone)]
struct SceneHit {
    node: SceneNodeId,
    rect: Rect,
}

#[derive(Debug, Clone)]
struct AssetHit {
    path: PathBuf,
    rect: Rect,
}

#[derive(Debug, Clone)]
struct CommandHit {
    command: CommandId,
    rect: Rect,
}

#[derive(Debug, Clone, Copy)]
struct MenuHit {
    menu: MenuId,
    rect: Rect,
}

#[derive(Debug, Clone, Copy)]
enum ActiveDrag {
    Splitter(Splitter),
    Tab { panel: PanelId, origin: DockRegion },
}

#[derive(Debug, Error)]
pub enum UiError {
    #[error("failed to prepare project directories: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to index assets: {0}")]
    Assets(#[from] crate::assets::AssetError),
    #[error("failed to load script: {0}")]
    Script(#[from] crate::script::ScriptError),
    #[error("failed to persist dock layout: {0}")]
    Dock(#[from] crate::dock::DockPersistenceError),
}

pub struct EditorState {
    project_root: PathBuf,
    pub layout: DockLayout,
    pub scene: SceneModel,
    pub assets: AssetIndex,
    pub script: ScriptDocument,
    highlighter: RustHighlighter,
    theme: Theme,
    repaint_requested: bool,
    play_mode: bool,
    focused_panel: Option<PanelId>,
    menu_open: Option<MenuId>,
    active_drag: Option<ActiveDrag>,
    pointer: Point,
    window_size: Size,
    tab_hits: Vec<TabHit>,
    region_hits: Vec<RegionHit>,
    scene_hits: Vec<SceneHit>,
    asset_hits: Vec<AssetHit>,
    command_hits: Vec<CommandHit>,
    menu_hits: Vec<MenuHit>,
    scroll_offsets: HashMap<PanelId, f32>,
    status: String,
}

impl EditorState {
    pub fn open(project_root: impl Into<PathBuf>) -> Result<Self, UiError> {
        let project_root = project_root.into();
        let assets_dir = project_root.join("assets");
        let scripts_dir = project_root.join("scripts");
        let layout_path = project_root.join(".crucible").join("editor-layout.ron");

        fs::create_dir_all(&assets_dir)?;
        fs::create_dir_all(&scripts_dir)?;
        ensure_default_script(&scripts_dir)?;

        let layout = DockLayout::load_from(&layout_path).unwrap_or_default();
        let assets = AssetIndex::scan(&assets_dir)?;
        let script_path = script_files(&scripts_dir)
            .into_iter()
            .next()
            .unwrap_or_else(|| scripts_dir.join("main.rs"));
        let script = ScriptDocument::load(script_path)?;
        let highlighter = RustHighlighter::new()?;

        Ok(Self {
            project_root,
            layout,
            scene: SceneModel::default(),
            assets,
            script,
            highlighter,
            theme: Theme::minimal_dark(),
            repaint_requested: true,
            play_mode: false,
            focused_panel: None,
            menu_open: None,
            active_drag: None,
            pointer: Point::new(0.0, 0.0),
            window_size: Size::new(1280.0, 720.0),
            tab_hits: Vec::new(),
            region_hits: Vec::new(),
            scene_hits: Vec::new(),
            asset_hits: Vec::new(),
            command_hits: Vec::new(),
            menu_hits: Vec::new(),
            scroll_offsets: HashMap::new(),
            status: "Ready".to_string(),
        })
    }

    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    #[must_use]
    pub fn needs_continuous_repaint(&self) -> bool {
        self.play_mode
    }

    #[must_use]
    pub fn play_mode(&self) -> bool {
        self.play_mode
    }

    pub fn take_repaint_request(&mut self) -> bool {
        let requested = self.repaint_requested;
        self.repaint_requested = false;
        requested
    }

    pub fn invalidate(&mut self) {
        self.repaint_requested = true;
    }

    pub fn draw(&mut self, size: Size) -> DrawList {
        self.window_size = size;
        self.clear_hits();
        self.layout.clamp_to_window(size);

        let rects = self.layout.layout_rects(size);
        let mut draw = DrawList::new();
        draw.rect(
            Rect::new(0.0, 0.0, size.width, size.height),
            self.theme.background,
        );

        self.draw_toolbar(&mut draw, rects.toolbar);
        self.draw_region(&mut draw, DockRegion::Left, rects.left);
        self.draw_region(&mut draw, DockRegion::Center, rects.center);
        self.draw_region(&mut draw, DockRegion::Right, rects.right);
        self.draw_region(&mut draw, DockRegion::Bottom, rects.bottom);

        draw.rect(rects.left_splitter, self.theme.background);
        draw.rect(rects.right_splitter, self.theme.background);
        draw.rect(rects.bottom_splitter, self.theme.background);
        draw.border(rects.left_splitter, self.theme.border, 1.0);
        draw.border(rects.right_splitter, self.theme.border, 1.0);
        draw.border(rects.bottom_splitter, self.theme.border, 1.0);

        if let Some(menu) = self.menu_open {
            self.draw_menu(&mut draw, menu);
        }

        draw
    }

    pub fn handle_pointer_move(&mut self, point: Point) {
        self.pointer = point;
        if let Some(ActiveDrag::Splitter(splitter)) = self.active_drag {
            self.layout
                .set_splitter_position(splitter, point, self.window_size);
            self.persist_layout_silent();
            self.invalidate();
        }
    }

    pub fn handle_pointer_down(&mut self, point: Point, button: PointerButton) {
        self.pointer = point;
        if button != PointerButton::Primary {
            return;
        }

        if let Some(splitter) = self.layout.hit_splitter(point, self.window_size) {
            self.active_drag = Some(ActiveDrag::Splitter(splitter));
            self.invalidate();
            return;
        }

        if let Some(hit) = self
            .command_hits
            .iter()
            .find(|hit| hit.rect.contains(point))
        {
            self.execute_command(hit.command);
            return;
        }

        if let Some(hit) = self
            .menu_hits
            .iter()
            .find(|hit| hit.rect.contains(point))
            .copied()
        {
            self.menu_open = if self.menu_open == Some(hit.menu) {
                None
            } else {
                Some(hit.menu)
            };
            self.invalidate();
            return;
        }

        if let Some(hit) = self
            .tab_hits
            .iter()
            .find(|hit| hit.rect.contains(point))
            .cloned()
        {
            self.layout.select_panel(hit.panel);
            self.focused_panel = Some(hit.panel);
            self.active_drag = Some(ActiveDrag::Tab {
                panel: hit.panel,
                origin: hit.region,
            });
            self.invalidate();
            return;
        }

        if let Some(hit) = self
            .scene_hits
            .iter()
            .find(|hit| hit.rect.contains(point))
            .cloned()
        {
            self.scene.select(hit.node);
            self.focused_panel = Some(PanelId::SceneOutline);
            self.invalidate();
            return;
        }

        if let Some(hit) = self
            .asset_hits
            .iter()
            .find(|hit| hit.rect.contains(point))
            .cloned()
        {
            self.assets.select(hit.path);
            self.focused_panel = Some(PanelId::AssetManager);
            self.invalidate();
            return;
        }

        if self
            .region_hits
            .iter()
            .any(|hit| hit.region == DockRegion::Bottom && hit.rect.contains(point))
            && self.layout.active_panel(DockRegion::Bottom) == Some(PanelId::ScriptEditor)
        {
            self.focused_panel = Some(PanelId::ScriptEditor);
            self.invalidate();
        }
    }

    pub fn handle_pointer_up(&mut self, point: Point, button: PointerButton) {
        self.pointer = point;
        if button != PointerButton::Primary {
            return;
        }

        if let Some(ActiveDrag::Tab { panel, origin }) = self.active_drag.take() {
            if let Some(target) = self
                .region_hits
                .iter()
                .find(|hit| hit.rect.contains(point))
                .map(|hit| hit.region)
            {
                if target != origin {
                    self.layout.move_tab(panel, target);
                    self.persist_layout_silent();
                }
            }
            self.invalidate();
            return;
        }

        self.active_drag = None;
        self.persist_layout_silent();
    }

    pub fn handle_scroll(&mut self, delta_y: f32) {
        let Some(panel) = self.panel_at(self.pointer) else {
            return;
        };
        let offset = self.scroll_offsets.entry(panel).or_insert(0.0);
        *offset = (*offset - delta_y).max(0.0);
        self.invalidate();
    }

    pub fn handle_text_input(&mut self, text: &str) {
        if self.focused_panel != Some(PanelId::ScriptEditor) {
            return;
        }

        let normalized = text.replace('\r', "\n");
        if normalized.chars().all(|ch| ch.is_control() && ch != '\n') {
            return;
        }
        self.script.buffer.insert_str(&normalized);
        self.refresh_script_highlights();
        self.status = "Script modified".to_string();
        self.invalidate();
    }

    pub fn handle_key(&mut self, key: UiKey, modifiers: KeyModifiers) {
        if key == UiKey::Save {
            self.execute_command(CommandId::SaveScript);
            return;
        }

        if self.focused_panel != Some(PanelId::ScriptEditor) {
            if key == UiKey::Escape {
                self.menu_open = None;
                self.invalidate();
            }
            return;
        }

        match key {
            UiKey::Backspace => self.script.buffer.backspace(),
            UiKey::Delete => self.script.buffer.delete(),
            UiKey::Enter => self.script.buffer.insert_str("\n"),
            UiKey::ArrowLeft => self.script.buffer.move_left(modifiers.shift),
            UiKey::ArrowRight => self.script.buffer.move_right(modifiers.shift),
            UiKey::Escape => self.focused_panel = None,
            UiKey::ArrowUp | UiKey::ArrowDown | UiKey::Save => {}
        }

        self.refresh_script_highlights();
        self.invalidate();
    }

    fn execute_command(&mut self, command: CommandId) {
        match command {
            CommandId::SaveLayout => {
                self.persist_layout_silent();
                self.status = "Layout saved".to_string();
            }
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
        self.menu_open = None;
        self.invalidate();
    }

    fn refresh_script_highlights(&mut self) {
        self.script.highlights = self.highlighter.refresh(self.script.buffer.text());
    }

    fn persist_layout_silent(&mut self) {
        let path = self
            .project_root
            .join(".crucible")
            .join("editor-layout.ron");
        if let Err(error) = self.layout.save_to(&path) {
            self.status = format!("Layout save failed: {error}");
        }
    }

    fn clear_hits(&mut self) {
        self.tab_hits.clear();
        self.region_hits.clear();
        self.scene_hits.clear();
        self.asset_hits.clear();
        self.command_hits.clear();
        self.menu_hits.clear();
    }

    fn panel_at(&self, point: Point) -> Option<PanelId> {
        for hit in &self.region_hits {
            if hit.rect.contains(point) {
                return self.layout.active_panel(hit.region);
            }
        }
        None
    }

    fn draw_toolbar(&mut self, draw: &mut DrawList, rect: Rect) {
        draw.rect(rect, self.theme.panel_header);
        draw.border(rect, self.theme.border, 1.0);
        draw.text(
            "Crucible",
            Point::new(12.0, 9.0),
            13.0,
            self.theme.text,
            rect,
        );

        let menus = [
            (MenuId::File, "File"),
            (MenuId::Edit, "Edit"),
            (MenuId::View, "View"),
            (MenuId::Assets, "Assets"),
            (MenuId::Build, "Build"),
            (MenuId::Run, "Run"),
            (MenuId::Help, "Help"),
        ];
        let mut x = 96.0;
        for (menu, label) in menus {
            let width = label.len() as f32 * 8.0 + 18.0;
            let menu_rect = Rect::new(x, 4.0, width, 26.0);
            if self.menu_open == Some(menu) {
                draw.rect(menu_rect, self.theme.accent_soft);
                draw.border(menu_rect, self.theme.accent, 1.0);
            }
            draw.text(
                label,
                Point::new(x + 9.0, 10.0),
                12.0,
                self.theme.text,
                menu_rect,
            );
            self.menu_hits.push(MenuHit {
                menu,
                rect: menu_rect,
            });
            x += width + 2.0;
        }

        let mut button_x = rect.right() - 256.0;
        for (command, label) in [
            (CommandId::SaveScript, "Save"),
            (CommandId::Play, "Play"),
            (CommandId::Pause, "Pause"),
            (CommandId::Stop, "Stop"),
        ] {
            let button = Rect::new(button_x, 5.0, 58.0, 24.0);
            let active = matches!(command, CommandId::Play) && self.play_mode;
            draw.rect(
                button,
                if active {
                    self.theme.accent_soft
                } else {
                    self.theme.panel_alt
                },
            );
            draw.border(
                button,
                if active {
                    self.theme.accent
                } else {
                    self.theme.border
                },
                1.0,
            );
            draw.text(
                label,
                Point::new(button.x + 12.0, button.y + 7.0),
                11.0,
                self.theme.text,
                button,
            );
            self.command_hits.push(CommandHit {
                command,
                rect: button,
            });
            button_x += 62.0;
        }
    }

    fn draw_menu(&mut self, draw: &mut DrawList, menu: MenuId) {
        let Some(menu_hit) = self.menu_hits.iter().find(|hit| hit.menu == menu).copied() else {
            return;
        };
        let items: &[(&str, CommandId)] = match menu {
            MenuId::File => &[
                ("Save Script", CommandId::SaveScript),
                ("Save Layout", CommandId::SaveLayout),
            ],
            MenuId::Assets => &[("Refresh Assets", CommandId::RefreshAssets)],
            MenuId::Run => &[
                ("Play", CommandId::Play),
                ("Pause", CommandId::Pause),
                ("Stop", CommandId::Stop),
            ],
            _ => &[],
        };

        let height = (items.len().max(1) as f32) * 26.0 + 8.0;
        let menu_rect = Rect::new(menu_hit.rect.x, menu_hit.rect.bottom() + 4.0, 168.0, height);
        draw.rect(menu_rect, self.theme.panel_alt);
        draw.border(menu_rect, self.theme.border_strong, 1.0);

        if items.is_empty() {
            draw.text(
                "No actions yet",
                Point::new(menu_rect.x + 10.0, menu_rect.y + 12.0),
                12.0,
                self.theme.text_muted,
                menu_rect,
            );
            return;
        }

        let mut y = menu_rect.y + 6.0;
        for (label, command) in items {
            let item_rect = Rect::new(menu_rect.x + 4.0, y, menu_rect.width - 8.0, 24.0);
            draw.text(
                *label,
                Point::new(item_rect.x + 8.0, item_rect.y + 7.0),
                12.0,
                self.theme.text,
                item_rect,
            );
            self.command_hits.push(CommandHit {
                command: *command,
                rect: item_rect,
            });
            y += 26.0;
        }
    }

    fn draw_region(&mut self, draw: &mut DrawList, region: DockRegion, rect: Rect) {
        draw.rect(rect, self.theme.panel);
        draw.border(rect, self.theme.border, 1.0);

        let tab_bar = Rect::new(rect.x, rect.y, rect.width, TAB_HEIGHT);
        self.region_hits.push(RegionHit {
            region,
            rect: tab_bar,
        });
        draw.rect(tab_bar, self.theme.panel_header);

        let tabs = self.layout.tabs(region).to_vec();
        let active = self.layout.active_panel(region);
        let mut tab_x = rect.x + 2.0;
        for panel in tabs {
            let width = (panel.title().len() as f32 * 7.0 + 28.0).clamp(96.0, 168.0);
            let tab_rect = Rect::new(tab_x, rect.y + 3.0, width, TAB_HEIGHT - 5.0);
            let is_active = active == Some(panel);
            draw.rect(
                tab_rect,
                if is_active {
                    self.theme.panel
                } else {
                    self.theme.panel_alt
                },
            );
            draw.border(
                tab_rect,
                if is_active {
                    self.theme.accent
                } else {
                    self.theme.border
                },
                1.0,
            );
            draw.text(
                panel.title(),
                Point::new(tab_rect.x + 10.0, tab_rect.y + 7.0),
                11.0,
                if is_active {
                    self.theme.text
                } else {
                    self.theme.text_muted
                },
                tab_rect,
            );
            self.tab_hits.push(TabHit {
                panel,
                region,
                rect: tab_rect,
            });
            tab_x += width + 2.0;
        }

        let content = rect.shrink(1.0, TAB_HEIGHT, 1.0, 1.0);
        self.region_hits.push(RegionHit {
            region,
            rect: content,
        });
        if let Some(panel) = active {
            match panel {
                PanelId::Viewport => self.draw_viewport(draw, content),
                PanelId::SceneOutline => self.draw_scene_outline(draw, content),
                PanelId::Inspector => self.draw_inspector(draw, content),
                PanelId::AssetManager => self.draw_asset_manager(draw, content),
                PanelId::ScriptEditor => self.draw_script_editor(draw, content),
            }
        }
    }

    fn draw_viewport(&self, draw: &mut DrawList, rect: Rect) {
        draw.rect(rect, Color::rgb_u8(9, 12, 16));
        let grid = Color::rgba(0.25, 0.32, 0.40, 0.28);
        let strong = Color::rgba(0.45, 0.58, 0.72, 0.35);
        let spacing = 36.0;
        let mut x = rect.x + (rect.width % spacing) * 0.5;
        while x < rect.right() {
            draw.line(
                Point::new(x, rect.y),
                Point::new(x, rect.bottom()),
                1.0,
                grid,
            );
            x += spacing;
        }
        let mut y = rect.y + (rect.height % spacing) * 0.5;
        while y < rect.bottom() {
            draw.line(
                Point::new(rect.x, y),
                Point::new(rect.right(), y),
                1.0,
                grid,
            );
            y += spacing;
        }
        draw.line(
            Point::new(rect.x, rect.y + rect.height * 0.5),
            Point::new(rect.right(), rect.y + rect.height * 0.5),
            1.0,
            strong,
        );
        draw.line(
            Point::new(rect.x + rect.width * 0.5, rect.y),
            Point::new(rect.x + rect.width * 0.5, rect.bottom()),
            1.0,
            strong,
        );
        draw.text(
            if self.play_mode {
                "Viewport - running"
            } else {
                "Viewport - edit mode"
            },
            Point::new(rect.x + 14.0, rect.y + 14.0),
            12.0,
            self.theme.text_muted,
            rect,
        );
    }

    fn draw_scene_outline(&mut self, draw: &mut DrawList, rect: Rect) {
        draw.rect(rect, self.theme.panel);
        draw.text(
            "Scene",
            Point::new(rect.x + PADDING, rect.y + PADDING),
            12.0,
            self.theme.text_muted,
            rect,
        );
        let scroll = *self
            .scroll_offsets
            .get(&PanelId::SceneOutline)
            .unwrap_or(&0.0);
        let mut y = rect.y + 30.0 - scroll;
        for row in self.scene.visible_rows() {
            if y + ROW_HEIGHT >= rect.y && y <= rect.bottom() {
                let row_rect = Rect::new(rect.x + 4.0, y, rect.width - 8.0, ROW_HEIGHT);
                if self.scene.selected() == Some(row.id) {
                    draw.rect(row_rect, self.theme.accent_soft);
                    draw.border(row_rect, self.theme.accent, 1.0);
                }
                let indent = row.depth as f32 * 16.0;
                let marker = if row.has_children {
                    if row.expanded { "v" } else { ">" }
                } else {
                    "-"
                };
                draw.text(
                    marker,
                    Point::new(row_rect.x + 6.0 + indent, row_rect.y + 6.0),
                    11.0,
                    self.theme.text_muted,
                    row_rect,
                );
                draw.text(
                    row.name,
                    Point::new(row_rect.x + 22.0 + indent, row_rect.y + 6.0),
                    12.0,
                    self.theme.text,
                    row_rect,
                );
                self.scene_hits.push(SceneHit {
                    node: row.id,
                    rect: row_rect,
                });
            }
            y += ROW_HEIGHT;
        }
    }

    fn draw_inspector(&self, draw: &mut DrawList, rect: Rect) {
        draw.rect(rect, self.theme.panel);
        draw.text(
            "Inspector",
            Point::new(rect.x + PADDING, rect.y + PADDING),
            12.0,
            self.theme.text_muted,
            rect,
        );
        let name = self.scene.selected_name().unwrap_or("None");
        let row = Rect::new(rect.x + 8.0, rect.y + 34.0, rect.width - 16.0, 28.0);
        draw.rect(row, self.theme.panel_alt);
        draw.border(row, self.theme.border, 1.0);
        draw.text(
            "Name",
            Point::new(row.x + 8.0, row.y + 8.0),
            11.0,
            self.theme.text_muted,
            row,
        );
        draw.text(
            name,
            Point::new(row.x + 76.0, row.y + 8.0),
            12.0,
            self.theme.text,
            row,
        );
        draw.text(
            "Transform, components, and script bindings will attach here.",
            Point::new(rect.x + 10.0, row.bottom() + 18.0),
            11.0,
            self.theme.text_muted,
            rect,
        );
    }

    fn draw_asset_manager(&mut self, draw: &mut DrawList, rect: Rect) {
        draw.rect(rect, self.theme.panel);
        let search = Rect::new(rect.x + 8.0, rect.y + 8.0, 260.0, 24.0);
        draw.rect(search, self.theme.panel_alt);
        draw.border(search, self.theme.border, 1.0);
        let filter = if self.assets.filter().is_empty() {
            "Filter assets"
        } else {
            self.assets.filter()
        };
        draw.text(
            filter,
            Point::new(search.x + 8.0, search.y + 7.0),
            11.0,
            self.theme.text_muted,
            search,
        );
        draw.text(
            &self.status,
            Point::new(rect.x + 284.0, rect.y + 14.0),
            11.0,
            self.theme.text_muted,
            rect,
        );

        let scroll = *self
            .scroll_offsets
            .get(&PanelId::AssetManager)
            .unwrap_or(&0.0);
        let mut y = rect.y + 42.0 - scroll;
        for item in self.assets.visible_items() {
            if y + ROW_HEIGHT >= rect.y && y <= rect.bottom() {
                let row = Rect::new(rect.x + 8.0, y, rect.width - 16.0, ROW_HEIGHT);
                if self.assets.selected() == Some(item.path.as_path()) {
                    draw.rect(row, self.theme.accent_soft);
                    draw.border(row, self.theme.accent, 1.0);
                }
                draw.text(
                    asset_kind_label(item.kind),
                    Point::new(row.x + 8.0, row.y + 6.0),
                    11.0,
                    self.theme.text_muted,
                    row,
                );
                draw.text(
                    &item.display_path,
                    Point::new(row.x + 36.0, row.y + 6.0),
                    12.0,
                    self.theme.text,
                    row,
                );
                self.asset_hits.push(AssetHit {
                    path: item.path.clone(),
                    rect: row,
                });
            }
            y += ROW_HEIGHT;
        }
    }

    fn draw_script_editor(&self, draw: &mut DrawList, rect: Rect) {
        draw.rect(rect, Color::rgb_u8(16, 19, 23));
        let file_tab = Rect::new(rect.x + 8.0, rect.y + 6.0, 180.0, 24.0);
        draw.rect(file_tab, self.theme.panel_alt);
        draw.border(
            file_tab,
            if self.focused_panel == Some(PanelId::ScriptEditor) {
                self.theme.accent
            } else {
                self.theme.border
            },
            1.0,
        );
        draw.text_mono(
            self.script.file_name(),
            Point::new(file_tab.x + 8.0, file_tab.y + 7.0),
            11.0,
            self.theme.text,
            file_tab,
        );

        let content = rect.shrink(8.0, 38.0, 8.0, 8.0);
        draw.rect(content, Color::rgb_u8(12, 14, 18));
        draw.border(content, self.theme.border, 1.0);

        let text = self.script.buffer.text();
        let line_offsets = line_start_offsets(text);
        let scroll = *self
            .scroll_offsets
            .get(&PanelId::ScriptEditor)
            .unwrap_or(&0.0);
        let line_height = 18.0;
        let char_width = 7.3;
        let gutter_width = 48.0;
        let mut y = content.y + 8.0 - scroll;

        for (line_index, start) in line_offsets.iter().copied().enumerate() {
            let raw_end = line_offsets
                .get(line_index + 1)
                .copied()
                .unwrap_or(text.len());
            let line_end = if raw_end > start && text.as_bytes().get(raw_end - 1) == Some(&b'\n') {
                raw_end - 1
            } else {
                raw_end
            };
            if y + line_height >= content.y && y <= content.bottom() {
                let row = Rect::new(content.x, y, content.width, line_height);
                draw.text_mono(
                    format!("{:>3}", line_index + 1),
                    Point::new(content.x + 8.0, y + 3.0),
                    11.0,
                    self.theme.text_muted,
                    row,
                );
                self.draw_highlighted_line(
                    draw,
                    text,
                    start,
                    line_end,
                    Point::new(content.x + gutter_width, y + 3.0),
                    char_width,
                    row,
                );
            }
            y += line_height;
        }

        if self.focused_panel == Some(PanelId::ScriptEditor) {
            let (line, col) = self.script.buffer.line_col();
            let caret_y = content.y + 8.0 - scroll + line as f32 * line_height;
            if caret_y >= content.y && caret_y <= content.bottom() {
                let caret_x = content.x + gutter_width + col as f32 * char_width;
                draw.rect(
                    Rect::new(caret_x, caret_y + 2.0, 1.0, line_height - 4.0),
                    self.theme.accent,
                );
            }
        }
    }

    fn draw_highlighted_line(
        &self,
        draw: &mut DrawList,
        text: &str,
        start: usize,
        end: usize,
        origin: Point,
        char_width: f32,
        bounds: Rect,
    ) {
        let mut cursor = start;
        let spans: Vec<HighlightSpan> = self
            .script
            .highlights
            .iter()
            .copied()
            .filter(|span| span.end > start && span.start < end)
            .collect();

        for span in spans {
            let span_start = span.start.max(start).min(end);
            let span_end = span.end.max(start).min(end);
            if cursor < span_start {
                self.draw_code_segment(
                    draw,
                    text,
                    cursor,
                    span_start,
                    origin,
                    char_width,
                    bounds,
                    self.theme.text,
                );
            }
            self.draw_code_segment(
                draw,
                text,
                span_start,
                span_end,
                origin,
                char_width,
                bounds,
                self.color_for_highlight(span.kind),
            );
            cursor = span_end;
        }

        if cursor < end {
            self.draw_code_segment(
                draw,
                text,
                cursor,
                end,
                origin,
                char_width,
                bounds,
                self.theme.text,
            );
        }
    }

    fn draw_code_segment(
        &self,
        draw: &mut DrawList,
        text: &str,
        start: usize,
        end: usize,
        origin: Point,
        char_width: f32,
        bounds: Rect,
        color: Color,
    ) {
        let Some(segment) = text.get(start..end) else {
            return;
        };
        if segment.is_empty() {
            return;
        }
        let col = text[..start]
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .chars()
            .count();
        draw.text_mono(
            segment,
            Point::new(origin.x + col as f32 * char_width, origin.y),
            12.0,
            color,
            bounds,
        );
    }

    fn color_for_highlight(&self, kind: HighlightKind) -> Color {
        match kind {
            HighlightKind::Keyword => self.theme.code_keyword,
            HighlightKind::String => self.theme.code_string,
            HighlightKind::Number => self.theme.code_number,
            HighlightKind::Comment => self.theme.code_comment,
            HighlightKind::Function => self.theme.code_function,
            HighlightKind::Type => self.theme.code_keyword,
        }
    }
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

fn asset_kind_label(kind: crate::AssetKind) -> &'static str {
    match kind {
        crate::AssetKind::Folder => "DIR",
        crate::AssetKind::Mesh => "3D",
        crate::AssetKind::Texture => "IMG",
        crate::AssetKind::Script => "RS",
        crate::AssetKind::Material => "MAT",
        crate::AssetKind::Audio => "AUD",
        crate::AssetKind::Other => "FILE",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repaint_flag_is_consumed() {
        let mut state =
            EditorState::open(std::env::temp_dir().join("crucible-ui-repaint-test")).unwrap();

        assert!(state.take_repaint_request());
        assert!(!state.take_repaint_request());
        state.invalidate();
        assert!(state.take_repaint_request());
    }

    #[test]
    fn pointer_hits_scene_rows_after_draw() {
        let mut state =
            EditorState::open(std::env::temp_dir().join("crucible-ui-hit-test")).unwrap();
        state.draw(Size::new(1280.0, 720.0));
        let first = state.scene_hits.first().unwrap().rect;

        state.handle_pointer_down(
            Point::new(first.x + 8.0, first.y + 8.0),
            PointerButton::Primary,
        );

        assert_eq!(state.scene.selected(), Some(state.scene_hits[0].node));
    }
}
