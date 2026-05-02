use std::{fs, path::Path};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Point, Rect, Size};

const TOOLBAR_HEIGHT: f32 = 34.0;
const SPLITTER_SIZE: f32 = 5.0;
const MIN_SIDE: f32 = 180.0;
const MIN_CENTER: f32 = 320.0;
const MIN_BOTTOM: f32 = 140.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanelId {
    Viewport,
    SceneOutline,
    Inspector,
    AssetManager,
    ScriptEditor,
}

impl PanelId {
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::SceneOutline => "Scene Outline",
            Self::Inspector => "Inspector",
            Self::AssetManager => "Assets",
            Self::ScriptEditor => "Script Editor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DockRegion {
    Left,
    Center,
    Right,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Splitter {
    LeftVertical,
    RightVertical,
    BottomHorizontal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockLayout {
    pub left_width: f32,
    pub right_width: f32,
    pub bottom_height: f32,
    pub left_tabs: Vec<PanelId>,
    pub center_tabs: Vec<PanelId>,
    pub right_tabs: Vec<PanelId>,
    pub bottom_tabs: Vec<PanelId>,
    pub active_left: usize,
    pub active_center: usize,
    pub active_right: usize,
    pub active_bottom: usize,
}

impl Default for DockLayout {
    fn default() -> Self {
        Self {
            left_width: 280.0,
            right_width: 300.0,
            bottom_height: 230.0,
            left_tabs: vec![PanelId::SceneOutline],
            center_tabs: vec![PanelId::Viewport],
            right_tabs: vec![PanelId::Inspector],
            bottom_tabs: vec![PanelId::AssetManager, PanelId::ScriptEditor],
            active_left: 0,
            active_center: 0,
            active_right: 0,
            active_bottom: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockRects {
    pub toolbar: Rect,
    pub left: Rect,
    pub center: Rect,
    pub right: Rect,
    pub bottom: Rect,
    pub left_splitter: Rect,
    pub right_splitter: Rect,
    pub bottom_splitter: Rect,
}

#[derive(Debug, Error)]
pub enum DockPersistenceError {
    #[error("failed to read dock layout: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse dock layout: {0}")]
    Parse(#[from] ron::error::SpannedError),
    #[error("failed to serialize dock layout: {0}")]
    Serialize(#[from] ron::Error),
}

impl DockLayout {
    #[must_use]
    pub fn layout_rects(&self, size: Size) -> DockRects {
        let mut layout = self.clone();
        layout.clamp_to_window(size);

        let width = size.width.max(1.0);
        let height = size.height.max(1.0);
        let content_y = TOOLBAR_HEIGHT;
        let content_height = (height - TOOLBAR_HEIGHT).max(1.0);
        let bottom_height = layout
            .bottom_height
            .min(content_height - MIN_CENTER)
            .max(0.0);
        let main_height = (content_height - bottom_height - SPLITTER_SIZE).max(1.0);
        let left_width = layout.left_width;
        let right_width = layout.right_width;
        let center_x = left_width + SPLITTER_SIZE;
        let center_width = (width - left_width - right_width - SPLITTER_SIZE * 2.0).max(1.0);
        let right_x = center_x + center_width + SPLITTER_SIZE;
        let bottom_y = content_y + main_height + SPLITTER_SIZE;

        DockRects {
            toolbar: Rect::new(0.0, 0.0, width, TOOLBAR_HEIGHT),
            left: Rect::new(0.0, content_y, left_width, main_height),
            center: Rect::new(center_x, content_y, center_width, main_height),
            right: Rect::new(right_x, content_y, right_width, main_height),
            bottom: Rect::new(0.0, bottom_y, width, bottom_height),
            left_splitter: Rect::new(left_width, content_y, SPLITTER_SIZE, main_height),
            right_splitter: Rect::new(
                right_x - SPLITTER_SIZE,
                content_y,
                SPLITTER_SIZE,
                main_height,
            ),
            bottom_splitter: Rect::new(0.0, content_y + main_height, width, SPLITTER_SIZE),
        }
    }

    pub fn clamp_to_window(&mut self, size: Size) {
        let available_width = size
            .width
            .max(MIN_CENTER + MIN_SIDE * 2.0 + SPLITTER_SIZE * 2.0);
        let max_left =
            (available_width - self.right_width - MIN_CENTER - SPLITTER_SIZE * 2.0).max(MIN_SIDE);
        self.left_width = self.left_width.clamp(MIN_SIDE, max_left);

        let max_right =
            (available_width - self.left_width - MIN_CENTER - SPLITTER_SIZE * 2.0).max(MIN_SIDE);
        self.right_width = self.right_width.clamp(MIN_SIDE, max_right);

        let content_height = (size.height - TOOLBAR_HEIGHT).max(MIN_CENTER + MIN_BOTTOM);
        let max_bottom = (content_height - MIN_CENTER - SPLITTER_SIZE).max(MIN_BOTTOM);
        self.bottom_height = self.bottom_height.clamp(MIN_BOTTOM, max_bottom);

        self.clamp_active_indices();
    }

    pub fn set_splitter_position(&mut self, splitter: Splitter, position: Point, size: Size) {
        match splitter {
            Splitter::LeftVertical => self.left_width = position.x,
            Splitter::RightVertical => self.right_width = size.width - position.x,
            Splitter::BottomHorizontal => self.bottom_height = size.height - position.y,
        }
        self.clamp_to_window(size);
    }

    #[must_use]
    pub fn hit_splitter(&self, point: Point, size: Size) -> Option<Splitter> {
        let rects = self.layout_rects(size);
        if rects.left_splitter.contains(point) {
            Some(Splitter::LeftVertical)
        } else if rects.right_splitter.contains(point) {
            Some(Splitter::RightVertical)
        } else if rects.bottom_splitter.contains(point) {
            Some(Splitter::BottomHorizontal)
        } else {
            None
        }
    }

    #[must_use]
    pub fn tabs(&self, region: DockRegion) -> &[PanelId] {
        match region {
            DockRegion::Left => &self.left_tabs,
            DockRegion::Center => &self.center_tabs,
            DockRegion::Right => &self.right_tabs,
            DockRegion::Bottom => &self.bottom_tabs,
        }
    }

    #[must_use]
    pub fn active_panel(&self, region: DockRegion) -> Option<PanelId> {
        let active = match region {
            DockRegion::Left => self.active_left,
            DockRegion::Center => self.active_center,
            DockRegion::Right => self.active_right,
            DockRegion::Bottom => self.active_bottom,
        };
        self.tabs(region).get(active).copied()
    }

    pub fn select_panel(&mut self, panel: PanelId) {
        for region in [
            DockRegion::Left,
            DockRegion::Center,
            DockRegion::Right,
            DockRegion::Bottom,
        ] {
            if let Some(index) = self
                .tabs(region)
                .iter()
                .position(|candidate| *candidate == panel)
            {
                self.set_active_index(region, index);
                return;
            }
        }
    }

    pub fn move_tab(&mut self, panel: PanelId, target: DockRegion) {
        self.remove_tab(panel);
        let (tabs, active) = self.tabs_and_active_mut(target);
        tabs.push(panel);
        *active = tabs.len().saturating_sub(1);
        self.clamp_active_indices();
    }

    pub fn save_to(&self, path: &Path) -> Result<(), DockPersistenceError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let pretty = PrettyConfig::new();
        let serialized = ron::ser::to_string_pretty(self, pretty)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    pub fn load_from(path: &Path) -> Result<Self, DockPersistenceError> {
        let content = fs::read_to_string(path)?;
        let mut layout: Self = ron::from_str(&content)?;
        layout.clamp_active_indices();
        Ok(layout)
    }

    fn remove_tab(&mut self, panel: PanelId) {
        for region in [
            DockRegion::Left,
            DockRegion::Center,
            DockRegion::Right,
            DockRegion::Bottom,
        ] {
            let (tabs, _) = self.tabs_and_active_mut(region);
            if let Some(index) = tabs.iter().position(|candidate| *candidate == panel) {
                tabs.remove(index);
            }
        }
    }

    fn set_active_index(&mut self, region: DockRegion, index: usize) {
        match region {
            DockRegion::Left => self.active_left = index,
            DockRegion::Center => self.active_center = index,
            DockRegion::Right => self.active_right = index,
            DockRegion::Bottom => self.active_bottom = index,
        }
        self.clamp_active_indices();
    }

    fn tabs_and_active_mut(&mut self, region: DockRegion) -> (&mut Vec<PanelId>, &mut usize) {
        match region {
            DockRegion::Left => (&mut self.left_tabs, &mut self.active_left),
            DockRegion::Center => (&mut self.center_tabs, &mut self.active_center),
            DockRegion::Right => (&mut self.right_tabs, &mut self.active_right),
            DockRegion::Bottom => (&mut self.bottom_tabs, &mut self.active_bottom),
        }
    }

    fn clamp_active_indices(&mut self) {
        self.active_left = clamp_index(self.active_left, self.left_tabs.len());
        self.active_center = clamp_index(self.active_center, self.center_tabs.len());
        self.active_right = clamp_index(self.active_right, self.right_tabs.len());
        self.active_bottom = clamp_index(self.active_bottom, self.bottom_tabs.len());
    }
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitters_clamp_to_minimum_panel_sizes() {
        let mut layout = DockLayout::default();
        let size = Size::new(960.0, 640.0);

        layout.set_splitter_position(Splitter::LeftVertical, Point::new(20.0, 0.0), size);
        assert_eq!(layout.left_width, MIN_SIDE);

        layout.set_splitter_position(Splitter::BottomHorizontal, Point::new(0.0, 630.0), size);
        assert_eq!(layout.bottom_height, MIN_BOTTOM);
    }

    #[test]
    fn tabs_can_move_between_regions() {
        let mut layout = DockLayout::default();

        layout.move_tab(PanelId::ScriptEditor, DockRegion::Right);

        assert!(!layout.bottom_tabs.contains(&PanelId::ScriptEditor));
        assert!(layout.right_tabs.contains(&PanelId::ScriptEditor));
        assert_eq!(
            layout.active_panel(DockRegion::Right),
            Some(PanelId::ScriptEditor)
        );
    }

    #[test]
    fn layout_serializes_round_trip() {
        let layout = DockLayout::default();
        let serialized = ron::ser::to_string(&layout).unwrap();
        let parsed: DockLayout = ron::from_str(&serialized).unwrap();

        assert_eq!(parsed.left_tabs, layout.left_tabs);
        assert_eq!(parsed.bottom_tabs, layout.bottom_tabs);
    }

    #[test]
    fn splitters_are_hit_tested() {
        let layout = DockLayout::default();
        let size = Size::new(1280.0, 720.0);
        let rects = layout.layout_rects(size);

        assert_eq!(
            layout.hit_splitter(Point::new(rects.left_splitter.x + 1.0, 100.0), size),
            Some(Splitter::LeftVertical)
        );
    }
}
