use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

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

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.buffer = ScriptBuffer::new(text);
        self.highlights.clear();
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

pub struct RustHighlighter;

impl RustHighlighter {
    pub fn new() -> Result<Self, ScriptError> {
        Ok(Self)
    }

    #[must_use]
    pub fn refresh(&mut self, source: &str) -> Vec<HighlightSpan> {
        let mut spans = Vec::new();
        collect_lexical_highlights(source, &mut spans);
        spans.sort_by_key(|span| (span.start, span.end));
        spans.dedup_by_key(|span| (span.start, span.end, span.kind));
        spans
    }
}

fn collect_lexical_highlights(source: &str, spans: &mut Vec<HighlightSpan>) {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut previous_word: Option<&str> = None;

    while index < bytes.len() {
        if source[index..].starts_with("//") {
            let end = source[index..]
                .find('\n')
                .map(|offset| index + offset)
                .unwrap_or(source.len());
            spans.push(HighlightSpan {
                start: index,
                end,
                kind: HighlightKind::Comment,
            });
            index = end;
            continue;
        }

        if bytes[index] == b'"' {
            let mut end = index + 1;
            while end < bytes.len() {
                if bytes[end] == b'\\' {
                    end = (end + 2).min(bytes.len());
                    continue;
                }
                if bytes[end] == b'"' {
                    end += 1;
                    break;
                }
                end += 1;
            }
            spans.push(HighlightSpan {
                start: index,
                end,
                kind: HighlightKind::String,
            });
            index = end;
            previous_word = None;
            continue;
        }

        if bytes[index].is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
                index += 1;
            }
            spans.push(HighlightSpan {
                start,
                end: index,
                kind: HighlightKind::Number,
            });
            previous_word = None;
            continue;
        }

        if bytes[index].is_ascii_alphabetic() || bytes[index] == b'_' {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            let word = &source[start..index];
            if previous_word == Some("fn") {
                spans.push(HighlightSpan {
                    start,
                    end: index,
                    kind: HighlightKind::Function,
                });
            } else if is_keyword(word) {
                spans.push(HighlightSpan {
                    start,
                    end: index,
                    kind: HighlightKind::Keyword,
                });
            } else if is_type_word(word) {
                spans.push(HighlightSpan {
                    start,
                    end: index,
                    kind: HighlightKind::Type,
                });
            }
            previous_word = Some(word);
            continue;
        }

        if !bytes[index].is_ascii_whitespace() {
            previous_word = None;
        }
        index += 1;
    }
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "let"
            | "fn"
            | "struct"
            | "impl"
            | "pub"
            | "use"
            | "mod"
            | "enum"
            | "trait"
            | "match"
            | "if"
            | "else"
            | "for"
            | "while"
            | "loop"
            | "return"
            | "Self"
    )
}

fn is_type_word(word: &str) -> bool {
    matches!(
        word,
        "bool"
            | "char"
            | "f32"
            | "f64"
            | "i32"
            | "i64"
            | "isize"
            | "str"
            | "String"
            | "u32"
            | "u64"
            | "usize"
    )
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
