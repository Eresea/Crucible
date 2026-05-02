use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Folder,
    Mesh,
    Texture,
    Script,
    Material,
    Audio,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetItem {
    pub path: PathBuf,
    pub display_path: String,
    pub kind: AssetKind,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AssetIndex {
    root: PathBuf,
    items: Vec<AssetItem>,
    selected: Option<PathBuf>,
    filter: String,
}

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("failed to create asset directory: {0}")]
    Create(#[from] std::io::Error),
}

impl AssetIndex {
    pub fn scan(root: impl Into<PathBuf>) -> Result<Self, AssetError> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        let mut index = Self {
            root,
            ..Self::default()
        };
        index.refresh()?;
        Ok(index)
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn items(&self) -> &[AssetItem] {
        &self.items
    }

    #[must_use]
    pub fn visible_items(&self) -> Vec<&AssetItem> {
        if self.filter.trim().is_empty() {
            return self.items.iter().collect();
        }

        let filter = self.filter.to_lowercase();
        self.items
            .iter()
            .filter(|item| item.display_path.to_lowercase().contains(&filter))
            .collect()
    }

    #[must_use]
    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    pub fn select(&mut self, path: impl Into<PathBuf>) {
        self.selected = Some(path.into());
    }

    pub fn set_filter(&mut self, filter: impl Into<String>) {
        self.filter = filter.into();
    }

    #[must_use]
    pub fn filter(&self) -> &str {
        &self.filter
    }

    pub fn refresh(&mut self) -> Result<(), AssetError> {
        self.items.clear();
        collect_assets(&self.root, &self.root, &mut self.items)?;
        self.items.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.display_path.cmp(&right.display_path))
        });
        Ok(())
    }
}

fn collect_assets(
    root: &Path,
    current: &Path,
    items: &mut Vec<AssetItem>,
) -> Result<(), AssetError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        let is_dir = metadata.is_dir();
        let display_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        items.push(AssetItem {
            kind: classify_asset(&path, is_dir),
            path: path.clone(),
            display_path,
            is_dir,
        });

        if is_dir {
            collect_assets(root, &path, items)?;
        }
    }
    Ok(())
}

fn classify_asset(path: &Path, is_dir: bool) -> AssetKind {
    if is_dir {
        return AssetKind::Folder;
    }

    match path.extension().and_then(|extension| extension.to_str()) {
        Some("glb" | "gltf" | "obj" | "fbx") => AssetKind::Mesh,
        Some("png" | "jpg" | "jpeg" | "tga" | "exr" | "hdr") => AssetKind::Texture,
        Some("rs" | "lua" | "rhai") => AssetKind::Script,
        Some("mat" | "material") => AssetKind::Material,
        Some("wav" | "ogg" | "mp3" | "flac") => AssetKind::Audio,
        _ => AssetKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn asset_scan_discovers_nested_files() {
        let root = std::env::temp_dir().join(format!(
            "crucible-ui-assets-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("models")).unwrap();
        fs::write(root.join("models").join("ship.glb"), "").unwrap();
        fs::write(root.join("albedo.png"), "").unwrap();

        let index = AssetIndex::scan(&root).unwrap();

        assert!(
            index
                .items()
                .iter()
                .any(|item| item.display_path == "models/ship.glb" && item.kind == AssetKind::Mesh)
        );
        assert!(
            index
                .items()
                .iter()
                .any(|item| item.display_path == "albedo.png" && item.kind == AssetKind::Texture)
        );

        fs::remove_dir_all(root).unwrap();
    }
}
