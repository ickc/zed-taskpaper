//! Passive features: diagnostics, inlay hints, hover.

use chrono::NaiveDate;
use lsp_types::{
    Diagnostic, DiagnosticSeverity, Hover, HoverContents, InlayHint, InlayHintLabel, MarkupContent,
    MarkupKind, Position,
};

use crate::dates;
use crate::model::{Doc, Kind, State};
use crate::util;

pub fn diagnostics(doc: &Doc, today: NaiveDate) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for item in &doc.items {
        let line = &doc.lines[item.row];

        // Indentation lint: TaskPaper's canonical indent is tabs.
        if line[..item.indent].contains(' ') {
            out.push(Diagnostic {
                range: util::range(item.row, line, 0, item.indent),
                severity: Some(DiagnosticSeverity::HINT),
                source: Some("taskpaper".into()),
                message: "TaskPaper indents with tabs".into(),
                ..Diagnostic::default()
            });
        }

        if item.state != State::Open {
            continue; // finished items are never overdue
        }
        for tag in &item.tags {
            let (message, severity) = match tag.name.as_str() {
                "today" => ("due today".to_owned(), DiagnosticSeverity::INFORMATION),
                "due" | "start" => {
                    let Some(date) = tag.value.as_deref().and_then(dates::parse) else {
                        continue;
                    };
                    if date > today {
                        continue;
                    }
                    let noun = if tag.name == "start" {
                        "startable"
                    } else {
                        "due"
                    };
                    if date == today {
                        (format!("{noun} today"), DiagnosticSeverity::INFORMATION)
                    } else {
                        (
                            format!("{} ({})", noun, dates::relative(date, today)),
                            DiagnosticSeverity::WARNING,
                        )
                    }
                }
                _ => continue,
            };
            out.push(Diagnostic {
                range: util::range(item.row, line, tag.start, tag.end),
                severity: Some(severity),
                source: Some("taskpaper".into()),
                message,
                ..Diagnostic::default()
            });
        }
    }
    out
}

pub fn inlay_hints(
    doc: &Doc,
    start_row: usize,
    end_row: usize,
    today: NaiveDate,
) -> Vec<InlayHint> {
    let mut out = Vec::new();
    for (i, item) in doc.items.iter().enumerate() {
        if item.row < start_row || item.row > end_row {
            continue;
        }
        let line = &doc.lines[item.row];

        // Task counts after project headings.
        if item.kind == Kind::Project {
            let (open, done, cancelled) = doc.counts(i);
            if open + done + cancelled > 0 {
                let label = if open > 0 {
                    format!("{open} open")
                } else {
                    "all done".to_owned()
                };
                out.push(hint(item.row, line, line.trim_end().len(), label));
            }
        }

        // Countdown after future @due tags on open items (past/today ones
        // are already surfaced as diagnostics).
        if item.state == State::Open {
            for tag in &item.tags {
                if tag.name != "due" {
                    continue;
                }
                if let Some(date) = tag.value.as_deref().and_then(dates::parse) {
                    if date > today {
                        out.push(hint(item.row, line, tag.end, dates::relative(date, today)));
                    }
                }
            }
        }
    }
    out
}

fn hint(row: usize, line: &str, byte: usize, label: String) -> InlayHint {
    InlayHint {
        position: Position::new(row as u32, util::utf16_col(line, byte)),
        label: InlayHintLabel::String(label),
        kind: None,
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: None,
        data: None,
    }
}

pub fn hover(doc: &Doc, row: usize, col: u32, today: NaiveDate) -> Option<Hover> {
    let line = doc.lines.get(row)?;
    let byte = util::byte_from_utf16(line, col);

    if let Some((i, t)) = doc.tag_at(row, byte) {
        let tag = &doc.items[i].tags[t];
        let date = tag.value.as_deref().and_then(dates::parse);
        let text = match (tag.name.as_str(), date) {
            ("due", Some(d)) => format!("**@due** {d} — {}", dates::relative(d, today)),
            ("start", Some(d)) => format!("**@start** {d} — {}", dates::relative(d, today)),
            ("done", Some(d)) => format!("**@done** — completed {}", dates::ago(d, today)),
            ("done", None) => "**@done** — completed".into(),
            ("cancelled", Some(d)) => {
                format!("**@cancelled** — cancelled {}", dates::ago(d, today))
            }
            ("cancelled", None) => "**@cancelled**".into(),
            _ => return None,
        };
        return Some(markdown_hover(
            text,
            util::range(row, line, tag.start, tag.end),
        ));
    }

    let i = doc.item_at_row(row)?;
    if doc.items[i].kind == Kind::Project {
        let (open, done, cancelled) = doc.counts(i);
        let total = open + done + cancelled;
        let mut parts = vec![format!("{open} open")];
        if done > 0 {
            parts.push(format!("{done} done"));
        }
        if cancelled > 0 {
            parts.push(format!("{cancelled} cancelled"));
        }
        let text = format!(
            "**{}** — {} ({total} task{})",
            doc.items[i].name,
            parts.join(", "),
            if total == 1 { "" } else { "s" },
        );
        return Some(markdown_hover(text, util::line_range(row, line)));
    }
    None
}

fn markdown_hover(value: String, range: lsp_types::Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: Some(range),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::parse;

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn overdue_and_today() {
        let doc = parse("- a @due(2026-07-01)\n- b @due(2026-07-05)\n- c @due(2026-07-09)\n- d @due(2026-07-01) @done\n- e @today\n");
        let diags = diagnostics(&doc, day("2026-07-05"));
        let messages: Vec<&str> = diags.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(messages, ["due (4 days overdue)", "due today", "due today"]);
    }

    #[test]
    fn hints_count_and_countdown() {
        let doc = parse("P:\n\t- a\n\t- b @done\n\t- c @due(2026-07-08)\nQ:\n");
        let hints = inlay_hints(&doc, 0, 10, day("2026-07-05"));
        let labels: Vec<String> = hints
            .iter()
            .map(|h| match &h.label {
                InlayHintLabel::String(s) => s.clone(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(labels, ["2 open", "in 3 days"]);
    }

    #[test]
    fn hover_project_counts() {
        let doc = parse("P:\n\t- a\n\t- b @done\n");
        let h = hover(&doc, 0, 1, day("2026-07-05")).unwrap();
        let HoverContents::Markup(m) = h.contents else {
            unreachable!()
        };
        assert_eq!(m.value, "**P** — 1 open, 1 done (2 tasks)");
    }
}
