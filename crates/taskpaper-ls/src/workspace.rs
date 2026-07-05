//! Workspace-wide features: the file index, tag completion, tag rename,
//! and project symbols.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use lsp_types::{
    CompletionItem, CompletionItemKind, Location, Position, Range, SymbolInformation, SymbolKind,
    TextEdit, Uri,
};

use crate::dates;
use crate::model::{self, Doc, Kind};
use crate::util;

/// Tags offered even in an empty workspace.
const BUILTIN_TAGS: &[&str] = &[
    "cancelled",
    "done",
    "due",
    "flagged",
    "priority",
    "search",
    "start",
    "today",
    "waiting",
];

#[derive(Default)]
pub struct Index {
    /// tag name -> occurrences, per file.
    tags: HashMap<PathBuf, HashMap<String, usize>>,
    /// (project name, row), per file.
    projects: HashMap<PathBuf, Vec<(String, u32)>>,
}

impl Index {
    pub fn build(root: Option<PathBuf>) -> Self {
        let mut index = Self::default();
        if let Some(root) = root {
            let mut stack = vec![(root, 0usize)];
            while let Some((dir, depth)) = stack.pop() {
                if depth > 8 {
                    continue;
                }
                let Ok(entries) = std::fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') || name == "node_modules" || name == "target" {
                        continue;
                    }
                    if path.is_dir() {
                        stack.push((path, depth + 1));
                    } else if path.extension().is_some_and(|e| e == "taskpaper") {
                        if let Ok(text) = std::fs::read_to_string(&path) {
                            index.update(&path, &text);
                        }
                    }
                }
            }
        }
        index
    }

    pub fn update(&mut self, path: &Path, text: &str) {
        let doc = model::parse(text);
        let mut tags: HashMap<String, usize> = HashMap::new();
        let mut projects = Vec::new();
        for item in &doc.items {
            for tag in &item.tags {
                *tags.entry(tag.name.clone()).or_default() += 1;
            }
            if item.kind == Kind::Project {
                projects.push((item.name.clone(), item.row as u32));
            }
        }
        self.tags.insert(path.to_owned(), tags);
        self.projects.insert(path.to_owned(), projects);
    }

    /// All known tag names with workspace-wide counts, builtins included.
    pub fn tag_names(&self) -> Vec<(String, usize)> {
        let mut merged: BTreeMap<String, usize> = BTreeMap::new();
        for name in BUILTIN_TAGS {
            merged.insert((*name).to_owned(), 0);
        }
        for tags in self.tags.values() {
            for (name, count) in tags {
                *merged.entry(name.clone()).or_default() += count;
            }
        }
        let mut out: Vec<(String, usize)> = merged.into_iter().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        out
    }

    pub fn files(&self) -> impl Iterator<Item = &PathBuf> {
        self.tags.keys()
    }

    #[allow(deprecated)]
    pub fn symbols(&self, query: &str) -> Vec<SymbolInformation> {
        let query = query.to_lowercase();
        let mut out = Vec::new();
        for (path, projects) in &self.projects {
            let Some(uri) = uri_of_path(path) else {
                continue;
            };
            for (name, row) in projects {
                if !query.is_empty() && !name.to_lowercase().contains(&query) {
                    continue;
                }
                out.push(SymbolInformation {
                    name: name.clone(),
                    kind: SymbolKind::NAMESPACE,
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: Range {
                            start: Position::new(*row, 0),
                            end: Position::new(*row, name.encode_utf16().count() as u32),
                        },
                    },
                    container_name: None,
                });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
}

pub fn path_of_uri(uri: &Uri) -> Option<PathBuf> {
    let s = uri.as_str();
    let rest = s.strip_prefix("file://")?;
    // Strip an authority component if present (usually empty).
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(i) => &rest[i..],
        None => return None,
    };
    // Percent-decode.
    let mut bytes = Vec::with_capacity(path.len());
    let mut iter = path.bytes();
    while let Some(b) = iter.next() {
        if b == b'%' {
            let hi = iter.next()?;
            let lo = iter.next()?;
            let hex = |c: u8| (c as char).to_digit(16);
            bytes.push((hex(hi)? * 16 + hex(lo)?) as u8);
        } else {
            bytes.push(b);
        }
    }
    Some(PathBuf::from(String::from_utf8(bytes).ok()?))
}

pub fn uri_of_path(path: &Path) -> Option<Uri> {
    let mut s = String::from("file://");
    for part in path.to_str()?.split('/') {
        if part.is_empty() {
            continue;
        }
        s.push('/');
        for byte in part.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    s.push(byte as char)
                }
                other => s.push_str(&format!("%{other:02X}")),
            }
        }
    }
    Uri::from_str(&s).ok()
}

/// Completions at a position: tag names after "@", or date suggestions
/// inside the parentheses of a date-bearing tag.
pub fn completions(doc: &Doc, index: &Index, row: usize, col: u32) -> Vec<CompletionItem> {
    let Some(line) = doc.lines.get(row) else {
        return Vec::new();
    };
    let byte = util::byte_from_utf16(line, col);
    let before = &line[..byte.min(line.len())];

    // Inside "@due(", "@start(", "@done(": offer dates.
    if let Some(open) = before.rfind('(') {
        let head = &before[..open];
        if !before[open + 1..].contains(')')
            && (head.ends_with("@due") || head.ends_with("@start") || head.ends_with("@done"))
        {
            let today = dates::today();
            return [
                (today, "today"),
                (today + chrono::Days::new(1), "tomorrow"),
                (today + chrono::Days::new(7), "in one week"),
            ]
            .into_iter()
            .enumerate()
            .map(|(i, (date, detail))| CompletionItem {
                label: date.to_string(),
                detail: Some(detail.to_owned()),
                kind: Some(CompletionItemKind::VALUE),
                sort_text: Some(format!("{i}")),
                ..CompletionItem::default()
            })
            .collect();
        }
    }

    // After "@" (possibly mid-name): offer tag names.
    let at = before.rfind('@');
    let is_tag_position = at.is_some_and(|at| {
        before[at + 1..]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == '-')
    });
    if !is_tag_position {
        return Vec::new();
    }
    index
        .tag_names()
        .into_iter()
        .enumerate()
        .map(|(i, (name, count))| CompletionItem {
            label: format!("@{name}"),
            filter_text: Some(name.clone()),
            insert_text: Some(name.clone()),
            detail: (count > 0).then(|| format!("{count} in workspace")),
            kind: Some(CompletionItemKind::KEYWORD),
            sort_text: Some(format!("{i:04}")),
            ..CompletionItem::default()
        })
        .collect()
}

/// The rename range for a tag name at a position (excluding the "@").
pub fn prepare_rename(doc: &Doc, row: usize, col: u32) -> Option<Range> {
    let line = doc.lines.get(row)?;
    let byte = util::byte_from_utf16(line, col);
    let (i, t) = doc.tag_at(row, byte)?;
    let tag = &doc.items[i].tags[t];
    if byte > tag.name_end {
        return None; // inside the value, not the name
    }
    Some(util::range(row, line, tag.start + 1, tag.name_end))
}

/// Rename a tag across the workspace. Open documents use their buffer
/// text; other indexed files are read from disk.
pub fn rename(
    index: &Index,
    open_docs: &HashMap<String, Doc>,
    old: &str,
    new: &str,
) -> HashMap<Uri, Vec<TextEdit>> {
    let mut changes: HashMap<Uri, Vec<TextEdit>> = HashMap::new();

    let mut do_doc = |uri: Uri, doc: &Doc| {
        let mut edits = Vec::new();
        for item in &doc.items {
            for tag in &item.tags {
                if tag.name == old {
                    edits.push(TextEdit {
                        range: util::range(
                            item.row,
                            &doc.lines[item.row],
                            tag.start + 1,
                            tag.name_end,
                        ),
                        new_text: new.to_owned(),
                    });
                }
            }
        }
        if !edits.is_empty() {
            changes.insert(uri, edits);
        }
    };

    for (uri_str, doc) in open_docs {
        if let Ok(uri) = Uri::from_str(uri_str) {
            do_doc(uri, doc);
        }
    }
    for path in index.files() {
        let Some(uri) = uri_of_path(path) else {
            continue;
        };
        if open_docs.contains_key(uri.as_str()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        do_doc(uri, &model::parse(&text));
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_roundtrip() {
        let path = PathBuf::from("/home/user/my lists/todo.taskpaper");
        let uri = uri_of_path(&path).unwrap();
        assert_eq!(uri.as_str(), "file:///home/user/my%20lists/todo.taskpaper");
        assert_eq!(path_of_uri(&uri).unwrap(), path);
    }

    #[test]
    fn tag_completion_position() {
        let doc = model::parse("- task @du\n");
        let index = Index::default();
        let items = completions(&doc, &index, 0, 10);
        assert!(items.iter().any(|c| c.label == "@due"));
        let none = completions(&doc, &index, 0, 4);
        assert!(none.is_empty());
    }

    #[test]
    fn date_completion_inside_due() {
        let doc = model::parse("- task @due(\n");
        let index = Index::default();
        let items = completions(&doc, &index, 0, 12);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].detail.as_deref(), Some("today"));
    }
}
