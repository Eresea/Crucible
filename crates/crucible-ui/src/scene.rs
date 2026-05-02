#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNodeId(pub u64);

#[derive(Debug, Clone)]
pub struct SceneNode {
    pub id: SceneNodeId,
    pub name: String,
    pub expanded: bool,
    pub children: Vec<SceneNode>,
}

impl SceneNode {
    #[must_use]
    pub fn new(id: SceneNodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
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

    pub fn toggle_expanded(&mut self, id: SceneNodeId) {
        if let Some(node) = find_mut(&mut self.roots, id) {
            node.expanded = !node.expanded;
        }
    }

    #[must_use]
    pub fn selected_name(&self) -> Option<&str> {
        self.selected
            .and_then(|id| self.find(id))
            .map(|node| node.name.as_str())
    }

    #[must_use]
    pub fn find(&self, id: SceneNodeId) -> Option<&SceneNode> {
        find(&self.roots, id)
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
