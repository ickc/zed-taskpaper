//! TaskPaper document model.
//!
//! Reimplements the line rules of the tree-sitter grammar in this repo
//! (tree-sitter-taskpaper/src/scanner.c) — keep the two in sync:
//!
//! * task: "-" followed by space/tab/EOL
//! * project: line ends with ":" once trailing whitespace and trailing
//!   @tags are stripped
//! * note: anything else non-blank
//! * tags:    only a trailing, whitespace-preceded run of @name/@name(value)
//! * @done/@cancelled state is inherited by the whole subtree
//! * indentation: one level per leading whitespace character (tabs
//!   canonical, spaces tolerated)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Project,
    Task,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Open,
    Done,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub value: Option<String>,
    /// Byte offsets within the raw line.
    pub start: usize,
    pub end: usize,
    pub name_end: usize,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub row: usize,
    /// Leading-whitespace character count (also byte count: ' ' and '\t').
    pub indent: usize,
    pub kind: Kind,
    /// Project name (no colon), task text (no bullet), or note text.
    pub name: String,
    /// Byte offset in the raw line where the body (after "- " for tasks)
    /// starts, and where it ends (before trailing whitespace/tags).
    pub body_start: usize,
    pub body_end: usize,
    pub tags: Vec<Tag>,
    pub state: State,
    pub parent: Option<usize>,
}

#[derive(Debug, Default)]
pub struct Doc {
    pub lines: Vec<String>,
    pub items: Vec<Item>,
}

fn is_tag_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' || !c.is_ascii()
}

fn is_inline_ws(c: char) -> bool {
    c == ' ' || c == '\t'
}

/// Split `content` (a line with indentation removed) into the byte length
/// of its body and its trailing tags, mirroring `body_end_without_tags` +
/// forward tag parsing in scanner.c. Tag offsets are relative to `content`.
pub fn split_trailing_tags(content: &str) -> (usize, Vec<Tag>) {
    let chars: Vec<(usize, char)> = content.char_indices().collect();
    let mut end = chars.len();
    while end > 0 && is_inline_ws(chars[end - 1].1) {
        end -= 1;
    }
    let mut tag_starts: Vec<usize> = Vec::new(); // char index of each '@'
    loop {
        let mut e = end;
        if e > 0 && chars[e - 1].1 == ')' {
            let mut p = e as isize - 2;
            while p >= 0 && chars[p as usize].1 != '(' && chars[p as usize].1 != ')' {
                p -= 1;
            }
            if p < 0 || chars[p as usize].1 != '(' {
                break;
            }
            e = p as usize;
        }
        let mut j = e;
        while j > 0 && is_tag_name_char(chars[j - 1].1) {
            j -= 1;
        }
        if j == e || j == 0 || chars[j - 1].1 != '@' {
            break;
        }
        if j - 1 > 0 && !is_inline_ws(chars[j - 2].1) {
            break;
        }
        tag_starts.push(j - 1);
        end = j - 1;
        while end > 0 && is_inline_ws(chars[end - 1].1) {
            end -= 1;
        }
    }
    let body_end = if end == 0 {
        0
    } else {
        chars[end - 1].0 + chars[end - 1].1.len_utf8()
    };

    tag_starts.reverse();
    let mut tags = Vec::new();
    for &at in &tag_starts {
        let mut j = at + 1;
        while j < chars.len() && is_tag_name_char(chars[j].1) {
            j += 1;
        }
        let name: String = chars[at + 1..j].iter().map(|&(_, c)| c).collect();
        let name_end_byte = byte_at(&chars, j, content);
        let (value, end_byte) = if j < chars.len() && chars[j].1 == '(' {
            let mut k = j + 1;
            while k < chars.len() && chars[k].1 != ')' {
                k += 1;
            }
            let value: String = chars[j + 1..k].iter().map(|&(_, c)| c).collect();
            (Some(value), byte_at(&chars, k + 1, content))
        } else {
            (None, name_end_byte)
        };
        tags.push(Tag {
            name,
            value,
            start: chars[at].0,
            end: end_byte,
            name_end: name_end_byte,
        });
    }
    (body_end, tags)
}

fn byte_at(chars: &[(usize, char)], idx: usize, content: &str) -> usize {
    if idx < chars.len() {
        chars[idx].0
    } else {
        content.len()
    }
}

pub fn parse(text: &str) -> Doc {
    let lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let mut items: Vec<Item> = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // indexes into items

    for (row, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|&c| is_inline_ws(c)).count();
        let content = &line[indent..];
        let (body_end, mut tags) = split_trailing_tags(content);
        let body = &content[..body_end];

        let is_task = content == "-"
            || content.starts_with("- ")
            || content.starts_with("-\t")
            || content == "-\r";
        let (kind, body_start, name) = if is_task {
            let text = body.get(2..).unwrap_or("").trim().to_owned();
            (Kind::Task, indent + 2.min(body.len()), text)
        } else if body.trim_end().ends_with(':') {
            let trimmed = body.trim_end();
            (
                Kind::Project,
                indent,
                trimmed[..trimmed.len() - 1].trim().to_owned(),
            )
        } else {
            (Kind::Note, indent, body.trim_end().to_owned())
        };

        // Shift tag offsets from content-relative to line-relative.
        for t in &mut tags {
            t.start += indent;
            t.end += indent;
            t.name_end += indent;
        }

        while let Some(&top) = stack.last() {
            if items[top].indent >= indent {
                stack.pop();
            } else {
                break;
            }
        }
        let parent = stack.last().copied();

        let own_cancelled = tags.iter().any(|t| t.name == "cancelled");
        let own_done = tags.iter().any(|t| t.name == "done");
        let inherited = parent.map(|p| items[p].state).unwrap_or(State::Open);
        let state = match inherited {
            State::Cancelled => State::Cancelled,
            State::Done => {
                if own_cancelled {
                    State::Cancelled
                } else {
                    State::Done
                }
            }
            State::Open => {
                if own_cancelled {
                    State::Cancelled
                } else if own_done {
                    State::Done
                } else {
                    State::Open
                }
            }
        };

        items.push(Item {
            row,
            indent,
            kind,
            name,
            body_start,
            body_end: indent + body_end,
            tags,
            state,
            parent,
        });
        stack.push(items.len() - 1);
    }

    Doc { lines, items }
}

impl Doc {
    /// Item indexes of `i`'s subtree, excluding `i` itself.
    pub fn subtree(&self, i: usize) -> std::ops::Range<usize> {
        let indent = self.items[i].indent;
        let mut end = i + 1;
        while end < self.items.len() && self.items[end].indent > indent {
            end += 1;
        }
        i + 1..end
    }

    /// (open, done, cancelled) task counts over `i` and its subtree.
    pub fn counts(&self, i: usize) -> (usize, usize, usize) {
        let mut open = 0;
        let mut done = 0;
        let mut cancelled = 0;
        for j in std::iter::once(i).chain(self.subtree(i)) {
            if self.items[j].kind == Kind::Task {
                match self.items[j].state {
                    State::Open => open += 1,
                    State::Done => done += 1,
                    State::Cancelled => cancelled += 1,
                }
            }
        }
        (open, done, cancelled)
    }

    pub fn item_at_row(&self, row: usize) -> Option<usize> {
        self.items.iter().position(|it| it.row == row)
    }

    /// The tag whose span contains the given line byte offset.
    pub fn tag_at(&self, row: usize, byte: usize) -> Option<(usize, usize)> {
        let i = self.item_at_row(row)?;
        let t = self.items[i]
            .tags
            .iter()
            .position(|t| t.start <= byte && byte < t.end)?;
        Some((i, t))
    }

    /// Names of ancestor projects, outermost first.
    pub fn project_path(&self, i: usize) -> Vec<String> {
        let mut path = Vec::new();
        let mut cur = self.items[i].parent;
        while let Some(p) = cur {
            if self.items[p].kind == Kind::Project {
                path.push(self.items[p].name.clone());
            }
            cur = self.items[p].parent;
        }
        path.reverse();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_and_tags() {
        let doc = parse("Home: @weekend\n\t- milk @due(2026-07-05) @done\n\tsee bob@example.com\n");
        assert_eq!(doc.items.len(), 3);
        assert_eq!(doc.items[0].kind, Kind::Project);
        assert_eq!(doc.items[0].name, "Home");
        assert_eq!(doc.items[0].tags.len(), 1);
        assert_eq!(doc.items[1].kind, Kind::Task);
        assert_eq!(doc.items[1].name, "milk");
        assert_eq!(doc.items[1].tags[0].value.as_deref(), Some("2026-07-05"));
        assert_eq!(doc.items[1].state, State::Done);
        assert_eq!(doc.items[2].kind, Kind::Note);
        assert!(doc.items[2].tags.is_empty(), "email is not a tag");
    }

    #[test]
    fn inheritance_and_counts() {
        let doc = parse("P: @done\n\t- a\n\t\t- b\n- c @cancelled\n\t- d\n- e\n");
        let states: Vec<State> = doc.items.iter().map(|it| it.state).collect();
        assert_eq!(
            states,
            [
                State::Done,
                State::Done,
                State::Done,
                State::Cancelled,
                State::Cancelled,
                State::Open
            ]
        );
        assert_eq!(doc.counts(0), (0, 2, 0));
    }

    #[test]
    fn mid_line_done_is_not_a_tag() {
        let doc = parse("- a @done b\n\t- child\n");
        assert!(doc.items[0].tags.is_empty());
        assert_eq!(doc.items[1].state, State::Open);
    }

    #[test]
    fn subtree_and_path() {
        let doc = parse("A:\n\tB:\n\t\t- t\n- top\n");
        assert_eq!(doc.subtree(0), 1..3);
        assert_eq!(doc.project_path(2), vec!["A", "B"]);
    }

    #[test]
    fn trailing_whitespace_and_colon_in_value() {
        let doc = parse("Errands: \n\t- meet @at(10:30)  \n");
        assert_eq!(doc.items[0].kind, Kind::Project);
        assert_eq!(doc.items[1].tags[0].value.as_deref(), Some("10:30"));
        assert_eq!(doc.items[1].name, "meet");
    }
}
