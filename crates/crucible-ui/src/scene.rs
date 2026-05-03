#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNodeId(pub u64);

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub id: SceneNodeId,
    pub name: String,
    pub transform: SceneTransform,
    pub visible: bool,
    pub expanded: bool,
    pub children: Vec<SceneNode>,
}

impl SceneNode {
    #[must_use]
    pub fn new(id: SceneNodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            transform: SceneTransform::default(),
            visible: true,
            expanded: true,
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_children(mut self, children: Vec<SceneNode>) -> Self {
        self.children = children;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTransform {
    pub translation: [f32; 3],
}

impl Default for SceneTransform {
    fn default() -> Self {
        Self {
            translation: [0.0, 0.0, 0.0],
        }
    }
}

#[derive(Debug, Clone)]
pub struct SceneRow {
    pub id: SceneNodeId,
    pub depth: usize,
    pub name: String,
    pub expanded: bool,
    pub has_children: bool,
}

#[derive(Debug, Clone)]
pub struct SceneModel {
    roots: Vec<SceneNode>,
    selected: Option<SceneNodeId>,
}

impl Default for SceneModel {
    fn default() -> Self {
        let camera = SceneNode::new(SceneNodeId(2), "Main Camera");
        let light = SceneNode::new(SceneNodeId(3), "Directional Light");
        let player = SceneNode::new(SceneNodeId(4), "Player")
            .with_children(vec![SceneNode::new(SceneNodeId(5), "Player Mesh")]);
        let world = SceneNode::new(SceneNodeId(1), "Sample Scene")
            .with_children(vec![camera, light, player]);

        Self {
            roots: vec![world],
            selected: Some(SceneNodeId(1)),
        }
    }
}

impl SceneModel {
    #[must_use]
    pub fn roots(&self) -> &[SceneNode] {
        &self.roots
    }

    #[must_use]
    pub fn selected(&self) -> Option<SceneNodeId> {
        self.selected
    }

    pub fn select(&mut self, id: SceneNodeId) {
        if self.find(id).is_some() {
            self.selected = Some(id);
        }
    }

    pub fn add_child_to_selected(&mut self) -> Option<SceneNodeId> {
        let id = self.selected?;
        let next_id = self.next_id();
        let parent = find_mut(&mut self.roots, id)?;
        let child = SceneNode::new(next_id, "New Node");
        parent.expanded = true;
        parent.children.push(child);
        self.selected = Some(next_id);
        Some(next_id)
    }

    pub fn duplicate_selected(&mut self) -> Option<SceneNodeId> {
        let id = self.selected?;
        let source = self.find(id)?.clone();
        let mut next_id = self.next_id().0;
        let duplicate = clone_with_new_ids(&source, &mut next_id);
        let duplicate_id = duplicate.id;

        if let Some(parent) = find_parent_mut(&mut self.roots, id) {
            let index = parent.children.iter().position(|node| node.id == id)?;
            parent.children.insert(index + 1, duplicate);
        } else {
            let index = self.roots.iter().position(|node| node.id == id)?;
            self.roots.insert(index + 1, duplicate);
        }

        self.selected = Some(duplicate_id);
        Some(duplicate_id)
    }

    pub fn delete_selected(&mut self) -> bool {
        let Some(id) = self.selected else {
            return false;
        };

        let removed = remove_node(&mut self.roots, id);
        if removed {
            self.selected = self.roots.first().map(|node| node.id);
        }
        removed
    }

    pub fn rename_selected(&mut self, name: impl Into<String>) {
        let Some(id) = self.selected else {
            return;
        };
        if let Some(node) = find_mut(&mut self.roots, id) {
            node.name = name.into();
        }
    }

    pub fn set_selected_translation_axis(&mut self, axis: usize, value: f32) {
        let Some(id) = self.selected else {
            return;
        };
        let Some(node) = find_mut(&mut self.roots, id) else {
            return;
        };
        if let Some(component) = node.transform.translation.get_mut(axis) {
            *component = value;
        }
    }

    pub fn set_selected_visible(&mut self, visible: bool) {
        let Some(id) = self.selected else {
            return;
        };
        if let Some(node) = find_mut(&mut self.roots, id) {
            node.visible = visible;
        }
    }

    pub fn reset_selected_transform(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        if let Some(node) = find_mut(&mut self.roots, id) {
            node.transform = SceneTransform::default();
        }
    }

    pub fn toggle_expanded(&mut self, id: SceneNodeId) {
        if let Some(node) = find_mut(&mut self.roots, id) {
            node.expanded = !node.expanded;
        }
    }

    pub fn toggle_selected_expanded(&mut self) {
        let Some(id) = self.selected else {
            return;
        };
        self.toggle_expanded(id);
    }

    #[must_use]
    pub fn selected_name(&self) -> Option<&str> {
        self.selected
            .and_then(|id| self.find(id))
            .map(|node| node.name.as_str())
    }

    #[must_use]
    pub fn selected_transform(&self) -> Option<SceneTransform> {
        self.selected
            .and_then(|id| self.find(id))
            .map(|node| node.transform)
    }

    #[must_use]
    pub fn selected_visible(&self) -> Option<bool> {
        self.selected
            .and_then(|id| self.find(id))
            .map(|node| node.visible)
    }

    #[must_use]
    pub fn find(&self, id: SceneNodeId) -> Option<&SceneNode> {
        find(&self.roots, id)
    }

    fn next_id(&self) -> SceneNodeId {
        SceneNodeId(max_id(&self.roots).unwrap_or(0) + 1)
    }

    #[must_use]
    pub fn visible_rows(&self) -> Vec<SceneRow> {
        let mut rows = Vec::new();
        for node in &self.roots {
            collect_rows(node, 0, &mut rows);
        }
        rows
    }
}

fn collect_rows(node: &SceneNode, depth: usize, rows: &mut Vec<SceneRow>) {
    rows.push(SceneRow {
        id: node.id,
        depth,
        name: node.name.clone(),
        expanded: node.expanded,
        has_children: !node.children.is_empty(),
    });

    if node.expanded {
        for child in &node.children {
            collect_rows(child, depth + 1, rows);
        }
    }
}

fn find(nodes: &[SceneNode], id: SceneNodeId) -> Option<&SceneNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(child) = find(&node.children, id) {
            return Some(child);
        }
    }
    None
}

fn find_mut(nodes: &mut [SceneNode], id: SceneNodeId) -> Option<&mut SceneNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(child) = find_mut(&mut node.children, id) {
            return Some(child);
        }
    }
    None
}

fn find_parent_mut(nodes: &mut [SceneNode], id: SceneNodeId) -> Option<&mut SceneNode> {
    for node in nodes {
        if node.children.iter().any(|child| child.id == id) {
            return Some(node);
        }
        if let Some(parent) = find_parent_mut(&mut node.children, id) {
            return Some(parent);
        }
    }
    None
}

fn remove_node(nodes: &mut Vec<SceneNode>, id: SceneNodeId) -> bool {
    if let Some(index) = nodes.iter().position(|node| node.id == id) {
        nodes.remove(index);
        return true;
    }

    for node in nodes {
        if remove_node(&mut node.children, id) {
            return true;
        }
    }
    false
}

fn max_id(nodes: &[SceneNode]) -> Option<u64> {
    nodes
        .iter()
        .map(|node| {
            let child_max = max_id(&node.children).unwrap_or(node.id.0);
            node.id.0.max(child_max)
        })
        .max()
}

fn clone_with_new_ids(node: &SceneNode, next_id: &mut u64) -> SceneNode {
    let id = SceneNodeId(*next_id);
    *next_id += 1;

    let mut duplicate = node.clone();
    duplicate.id = id;
    duplicate.name = format!("{} Copy", node.name);
    duplicate.children = node
        .children
        .iter()
        .map(|child| clone_with_new_ids(child, next_id))
        .collect();
    duplicate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_selection_tracks_existing_nodes() {
        let mut scene = SceneModel::default();

        scene.select(SceneNodeId(4));

        assert_eq!(scene.selected(), Some(SceneNodeId(4)));
        assert_eq!(scene.selected_name(), Some("Player"));
    }
}
