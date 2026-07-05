//! Editing features: code actions (toggle @done/@cancelled, archive, sort,
//! task/note conversion) and document formatting. All edits are computed as
//! text and applied through the LSP, so they operate on the buffer — never
//! directly on the file.

use chrono::NaiveDate;
use lsp_types::{CodeAction, CodeActionKind, TextEdit, Uri, WorkspaceEdit};
use std::collections::HashMap;

use crate::dates;
use crate::model::{Doc, Kind, State};
use crate::util;

pub fn code_actions(doc: &Doc, uri: &Uri, row: usize, today: NaiveDate) -> Vec<CodeAction> {
    let mut out = Vec::new();
    if let Some(i) = doc.item_at_row(row) {
        out.extend(toggle_action(doc, uri, i, "done", Some(today)));
        out.extend(toggle_action(doc, uri, i, "cancelled", None));
        out.extend(convert_action(doc, uri, i));
        out.extend(sort_action(doc, uri, i));
    }
    out.extend(archive_action(doc, uri));
    out
}

fn line_edit(doc: &Doc, uri: &Uri, row: usize, new_line: String) -> WorkspaceEdit {
    let edit = TextEdit {
        range: util::line_range(row, &doc.lines[row]),
        new_text: new_line,
    };
    WorkspaceEdit {
        changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
        ..WorkspaceEdit::default()
    }
}

fn action(title: String, kind: CodeActionKind, edit: WorkspaceEdit) -> CodeAction {
    CodeAction {
        title,
        kind: Some(kind),
        edit: Some(edit),
        ..CodeAction::default()
    }
}

/// Toggle a trailing state tag on the item's line. Adding @done stamps
/// today's date, matching the TaskPaper app; @cancelled stays bare.
fn toggle_action(
    doc: &Doc,
    uri: &Uri,
    i: usize,
    tag_name: &str,
    stamp: Option<NaiveDate>,
) -> Option<CodeAction> {
    let item = &doc.items[i];
    let line = &doc.lines[item.row];
    let eol = if line.ends_with('\r') { "\r" } else { "" };
    let body = line.trim_end();

    if let Some(tag) = item.tags.iter().find(|t| t.name == tag_name) {
        // Remove the tag along with the whitespace before it.
        let mut start = tag.start;
        while start > 0 && line.as_bytes()[start - 1].is_ascii_whitespace() {
            start -= 1;
        }
        let new_line = format!("{}{}{eol}", &line[..start], line[tag.end..].trim_end());
        Some(action(
            format!("Remove @{tag_name}"),
            CodeActionKind::REFACTOR_REWRITE,
            line_edit(doc, uri, item.row, new_line),
        ))
    } else {
        let suffix = match stamp {
            Some(date) => format!(" @{tag_name}({date})"),
            None => format!(" @{tag_name}"),
        };
        Some(action(
            format!("Mark @{tag_name}"),
            CodeActionKind::REFACTOR_REWRITE,
            line_edit(doc, uri, item.row, format!("{body}{suffix}{eol}")),
        ))
    }
}

fn convert_action(doc: &Doc, uri: &Uri, i: usize) -> Option<CodeAction> {
    let item = &doc.items[i];
    let line = &doc.lines[item.row];
    let indent = &line[..item.indent];
    let content = line[item.indent..].trim_end_matches('\r');
    let eol = if line.ends_with('\r') { "\r" } else { "" };
    let (title, new_content) = match item.kind {
        Kind::Task => (
            "Convert task to note",
            content
                .strip_prefix("- ")
                .or_else(|| content.strip_prefix("-\t"))
                .unwrap_or(content.strip_prefix('-').unwrap_or(content))
                .to_owned(),
        ),
        Kind::Note => ("Convert note to task", format!("- {content}")),
        Kind::Project => return None,
    };
    Some(action(
        title.to_owned(),
        CodeActionKind::REFACTOR_REWRITE,
        line_edit(doc, uri, item.row, format!("{indent}{new_content}{eol}")),
    ))
}

/// Rows `first..=last` covered by item `i`'s subtree block (including any
/// blank lines inside, excluding trailing blank lines).
fn block_rows(doc: &Doc, i: usize) -> (usize, usize) {
    let sub = doc.subtree(i);
    let last = if sub.is_empty() {
        doc.items[i].row
    } else {
        doc.items[sub.end - 1].row
    };
    (doc.items[i].row, last)
}

fn sort_action(doc: &Doc, uri: &Uri, i: usize) -> Option<CodeAction> {
    let children: Vec<usize> = doc
        .subtree(i)
        .filter(|&j| doc.items[j].parent == Some(i))
        .collect();
    if children.len() < 2 {
        return None;
    }
    let mut blocks: Vec<(Option<NaiveDate>, usize, usize)> = children
        .iter()
        .map(|&j| {
            let due = doc.items[j]
                .tags
                .iter()
                .find(|t| t.name == "due")
                .and_then(|t| t.value.as_deref())
                .and_then(dates::parse);
            let (first, last) = block_rows(doc, j);
            (due, first, last)
        })
        .collect();
    let already_sorted = blocks.windows(2).all(|w| {
        matches!(
            (w[0].0, w[1].0),
            (Some(a), Some(b)) if a <= b
        ) || w[1].0.is_none()
    });
    if already_sorted {
        return None;
    }
    let region_start = blocks.first()?.1;
    let region_end = blocks.last()?.2;
    // Stable sort: undated blocks keep their order at the end.
    blocks.sort_by_key(|&(due, first, _)| (due.unwrap_or(NaiveDate::MAX), first));

    let mut new_lines: Vec<String> = Vec::new();
    for &(_, first, last) in &blocks {
        new_lines.extend(doc.lines[first..=last].iter().cloned());
    }
    let edit = TextEdit {
        range: lsp_types::Range {
            start: lsp_types::Position::new(region_start as u32, 0),
            end: lsp_types::Position::new(
                region_end as u32,
                util::utf16_col(&doc.lines[region_end], doc.lines[region_end].len()),
            ),
        },
        new_text: new_lines.join("\n"),
    };
    Some(action(
        "Sort children by @due".to_owned(),
        CodeActionKind::REFACTOR_REWRITE,
        WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
            ..WorkspaceEdit::default()
        },
    ))
}

/// True if `i` sits inside the top-level "Archive" project.
fn in_archive(doc: &Doc, i: usize) -> bool {
    let mut cur = Some(i);
    while let Some(j) = cur {
        let it = &doc.items[j];
        if it.kind == Kind::Project && it.name == "Archive" && it.parent.is_none() {
            return true;
        }
        cur = it.parent;
    }
    false
}

/// Items whose whole subtree should move: finished, top-most (parent open
/// or absent), and not already archived.
fn archivable(doc: &Doc) -> Vec<usize> {
    (0..doc.items.len())
        .filter(|&i| {
            let it = &doc.items[i];
            it.state != State::Open
                && it.parent.is_none_or(|p| doc.items[p].state == State::Open)
                && !in_archive(doc, i)
        })
        .collect()
}

fn archive_action(doc: &Doc, uri: &Uri) -> Option<CodeAction> {
    let new_text = archive(doc)?;
    let edit = TextEdit {
        range: util::full_range(&doc.lines),
        new_text,
    };
    Some(action(
        "Archive finished items".to_owned(),
        CodeActionKind::SOURCE,
        WorkspaceEdit {
            changes: Some(HashMap::from([(uri.clone(), vec![edit])])),
            ..WorkspaceEdit::default()
        },
    ))
}

/// New document text with every finished top-most subtree moved to the top
/// of a top-level "Archive:" project (created at the end if missing). Each
/// moved item is stamped with a `@project(A / B)` breadcrumb so the parent
/// chain survives the move — the convention the TaskPaper 3 app uses.
pub fn archive(doc: &Doc) -> Option<String> {
    let targets = archivable(doc);
    if targets.is_empty() {
        return None;
    }

    let mut moved: Vec<bool> = vec![false; doc.lines.len()];
    let mut blocks: Vec<Vec<String>> = Vec::new();
    for &i in &targets {
        let item = &doc.items[i];
        let (first, last) = block_rows(doc, i);
        let mut block = Vec::new();
        for (row, line) in doc.lines.iter().enumerate().take(last + 1).skip(first) {
            moved[row] = true;
            block.push(reindent(line, item.indent, 1));
        }
        let path = doc.project_path(i);
        if !path.is_empty() {
            let first_line = &mut block[0];
            let eol = if first_line.ends_with('\r') { "\r" } else { "" };
            *first_line = format!(
                "{} @project({}){eol}",
                first_line.trim_end(),
                path.join(" / ")
            );
        }
        blocks.push(block);
    }

    let archive_row = doc
        .items
        .iter()
        .find(|it| it.kind == Kind::Project && it.name == "Archive" && it.parent.is_none())
        .map(|it| it.row);

    let mut out: Vec<String> = Vec::new();
    for (row, line) in doc.lines.iter().enumerate() {
        if moved[row] {
            continue;
        }
        out.push(line.clone());
        if Some(row) == archive_row {
            for block in &blocks {
                out.extend(block.iter().cloned());
            }
        }
    }
    if archive_row.is_none() {
        // Keep a single trailing empty line (from the final newline) last.
        let trailing_empty = out.last().is_some_and(|l| l.is_empty());
        if trailing_empty {
            out.pop();
        }
        out.push("Archive:".to_owned());
        for block in &blocks {
            out.extend(block.iter().cloned());
        }
        if trailing_empty {
            out.push(String::new());
        }
    }
    Some(out.join("\n"))
}

/// Re-indent a line from a subtree based at `old_base` to `new_base`,
/// converting the leading run to tabs. Blank lines pass through.
fn reindent(line: &str, old_base: usize, new_base: usize) -> String {
    if line.trim().is_empty() {
        return line.to_owned();
    }
    let indent = line.chars().take_while(|&c| c == ' ' || c == '\t').count();
    let depth = indent.saturating_sub(old_base) + new_base;
    format!("{}{}", "\t".repeat(depth), &line[indent..])
}

/// Formatting: strip trailing whitespace, normalize the gap between body
/// and tags (and between tags) to one space, convert leading whitespace to
/// tabs (one per character, matching the grammar's one-column-per-character
/// indent rule).
pub fn format(doc: &Doc) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    for item in &doc.items {
        let line = &doc.lines[item.row];
        let eol = if line.trim_end_matches('\n').ends_with('\r') {
            "\r"
        } else {
            ""
        };
        let indent = "\t".repeat(item.indent);
        let body = line[item.body_start.min(line.len())..item.body_end].trim_end();
        let mut new_line = match item.kind {
            Kind::Task if body.is_empty() => format!("{indent}-"),
            Kind::Task => format!("{indent}- {body}"),
            _ => format!("{indent}{body}"),
        };
        for tag in &item.tags {
            new_line.push(' ');
            new_line.push_str(line[tag.start..tag.end].trim_end());
        }
        new_line.push_str(eol);
        if new_line != *line {
            edits.push(TextEdit {
                range: util::line_range(item.row, line),
                new_text: new_line.trim_end_matches('\r').to_owned() + eol,
            });
        }
    }
    // Also strip whitespace-only lines down to empty.
    for (row, line) in doc.lines.iter().enumerate() {
        let stripped = line.trim_end_matches('\r');
        if stripped.trim().is_empty() && !stripped.is_empty() {
            edits.push(TextEdit {
                range: util::line_range(row, line),
                new_text: String::new(),
            });
        }
    }
    edits.sort_by_key(|e| e.range.start.line);
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse;

    #[test]
    fn archive_moves_subtree_with_breadcrumb() {
        let doc = parse("Home:\n\t- keep\n\t- done one @done\n\t\tsub note\nArchive:\n\t- older @done @project(Home)\n");
        let out = archive(&doc).unwrap();
        assert_eq!(
            out,
            "Home:\n\t- keep\nArchive:\n\t- done one @done @project(Home)\n\t\tsub note\n\t- older @done @project(Home)\n"
        );
    }

    #[test]
    fn archive_creates_project_and_deep_reindent() {
        let doc = parse("A:\n\tB:\n\t\t- x @done\n\t\t\t- child\n");
        let out = archive(&doc).unwrap();
        assert_eq!(
            out,
            "A:\n\tB:\nArchive:\n\t- x @done @project(A / B)\n\t\t- child\n"
        );
    }

    #[test]
    fn archive_skips_nested_done_inside_done() {
        // Only the top-most finished item moves; its subtree goes with it.
        let doc = parse("- a @done\n\t- b @done\n");
        let out = archive(&doc).unwrap();
        assert_eq!(out, "Archive:\n\t- a @done\n\t\t- b @done\n");
    }

    #[test]
    fn archive_nothing_to_do() {
        let doc = parse("- open\n");
        assert!(archive(&doc).is_none());
    }

    #[test]
    fn format_normalizes() {
        let doc = parse("P:  \n  - task   @due(x)  @done\n");
        let edits = format(&doc);
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].new_text, "P:");
        assert_eq!(edits[1].new_text, "\t\t- task @due(x) @done");
    }

    #[test]
    fn format_empty_task_body() {
        // A bare "-" and a tag-only task are already canonical: no double
        // space before the tag, no trailing space after the marker.
        let doc = parse("- @done\n-\n");
        assert!(format(&doc).is_empty());
        let doc = parse("-  @done\n- \n");
        let edits = format(&doc);
        let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
        assert_eq!(texts, ["- @done", "-"]);
    }
}
