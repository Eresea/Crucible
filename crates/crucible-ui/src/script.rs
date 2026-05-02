use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HighlightSpan {
    pub start: usize,
    pub end: usize,
    pub kind: HighlightKind,
}

#[derive(Debug, Clone)]
pub struct ScriptBuffer {
    text: String,
    caret: usize,
    selection_anchor: Option<usize>,
}

impl ScriptBuffer {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            caret: text.len(),
            text,
            selection_anchor: None,
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn caret(&self) -> usize {
        self.caret
    }

    #[must_use]
    pub fn selection_anchor(&self) -> Option<usize> {
        self.selection_anchor
    }

    pub fn insert_str(&mut self, text: &str) {
        self.delete_selection();
        self.text.insert_str(self.caret, text);
        self.caret += text.len();
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret == 0 {
            return;
        }

        let previous = previous_boundary(&self.text, self.caret);
        self.text.replace_range(previous..self.caret, "");
        self.caret = previous;
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.caret >= self.text.len() {
            return;
        }

        let next = next_boundary(&self.text, self.caret);
        self.text.replace_range(self.caret..next, "");
    }

    pub fn move_left(&mut self, selecting: bool) {
        self.update_selection_anchor(selecting);
        self.caret = previous_boundary(&self.text, self.caret);
    }

    pub fn move_right(&mut self, selecting: bool) {
        self.update_selection_anchor(selecting);
        self.caret = next_boundary(&self.text, self.caret);
    }

    #[must_use]
    pub fn line_col(&self) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for (index, ch) in self.text.char_indices() {
            if index >= self.caret {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn update_selection_anchor(&mut self, selecting: bool) {
        if selecting {
            self.selection_anchor.get_or_insert(self.caret);
        } else {
            self.selection_anchor = None;
        }
    }

    fn delete_selection(&mut self) -> bool {
        let Some(anchor) = self.selection_anchor.take() else {
            return false;
        };
        let start = anchor.min(self.caret);
        let end = anchor.max(self.caret);
        if start == end {
            return false;
        }
        self.text.replace_range(start..end, "");
        self.caret = start;
        true
    }
}

#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("failed to read script: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to initialize Rust parser")]
    Parser,
}

pub struct ScriptDocument {
    pub path: PathBuf,
    pub buffer: ScriptBuffer,
    pub highlights: Vec<HighlightSpan>,
}

impl ScriptDocument {
    pub fn load(path: impl Into<PathBuf>) -> Result<Self, ScriptError> {
        let path = path.into();
        let text = fs::read_to_string(&path)?;
        let mut highlighter = RustHighlighter::new()?;
        let highlights = highlighter.refresh(&text);
        Ok(Self {
            path,
            buffer: ScriptBuffer::new(text),
            highlights,
        })
    }

    pub fn save(&self) -> Result<(), ScriptError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, self.buffer.text())?;
        Ok(())
    }

    #[must_use]
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("untitled.rs")
            .to_string()
    }
}

pub struct RustHighlighter {
    parser: Parser,
}

impl RustHighlighter {
    pub fn new() -> Result<Self, ScriptError> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|_| ScriptError::Parser)?;
        Ok(Self { parser })
    }

    #[must_use]
    pub fn refresh(&mut self, text: &str) -> Vec<HighlightSpan> {
        let Some(tree) = self.parser.parse(text, None) else {
            return Vec::new();
        };

        let mut spans = Vec::new();
        collect_highlights(tree.root_node(), &mut spans);
        spans.sort_by_key(|span| (span.start, span.end));
        spans.dedup_by_key(|span| (span.start, span.end, span.kind));
        spans
    }
}

fn collect_highlights(node: Node<'_>, spans: &mut Vec<HighlightSpan>) {
    if node.kind() == "function_item" {
        if let Some(name) = node.child_by_field_name("name") {
            spans.push(HighlightSpan {
                start: name.start_byte(),
                end: name.end_byte(),
                kind: HighlightKind::Function,
            });
        }
    }

    if let Some(kind) = classify_node(node.kind()) {
        spans.push(HighlightSpan {
            start: node.start_byte(),
            end: node.end_byte(),
            kind,
        });
        return;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_highlights(child, spans);
    }
}

fn classify_node(kind: &str) -> Option<HighlightKind> {
    match kind {
        "line_comment" | "block_comment" => Some(HighlightKind::Comment),
        "string_literal" | "raw_string_literal" | "char_literal" => Some(HighlightKind::String),
        "integer_literal" | "float_literal" => Some(HighlightKind::Number),
        "primitive_type" | "type_identifier" => Some(HighlightKind::Type),
        "let" | "fn" | "struct" | "impl" | "pub" | "use" | "mod" | "enum" | "trait" | "match"
        | "if" | "else" | "for" | "while" | "loop" | "return" | "Self" => {
            Some(HighlightKind::Keyword)
        }
        _ => None,
    }
}

fn previous_boundary(text: &str, index: usize) -> usize {
    if index == 0 {
        return 0;
    }
    text[..index]
        .char_indices()
        .last()
        .map(|(position, _)| position)
        .unwrap_or(0)
}

fn next_boundary(text: &str, index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    text[index..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| index + offset)
        .unwrap_or(text.len())
}

#[must_use]
pub fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            offsets.push(index + 1);
        }
    }
    offsets
}

#[must_use]
pub fn script_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_buffer_insert_delete_and_caret_work() {
        let mut buffer = ScriptBuffer::new("fn main() {}");

        buffer.move_left(false);
        buffer.backspace();
        buffer.insert_str("println!();");

        assert!(buffer.text().contains("println!();"));
        assert!(buffer.caret() <= buffer.text().len());
    }

    #[test]
    fn highlighter_finds_rust_function_spans() {
        let mut highlighter = RustHighlighter::new().unwrap();
        let spans = highlighter.refresh("pub fn main() { let value = 1; }");

        assert!(
            spans
                .iter()
                .any(|span| span.kind == HighlightKind::Function)
        );
        assert!(spans.iter().any(|span| span.kind == HighlightKind::Number));
    }
}
