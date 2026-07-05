# zed-taskpaper

[TaskPaper](https://www.taskpaper.com) language support for [Zed](https://zed.dev):
syntax highlighting, `@done`/`@cancelled` fading, and outline navigation for
plain-text todo lists. A pure language extension — tree-sitter grammar plus
queries, no language server, no background processes.

## Format

- **Task** — a line starting with `- `.
- **Project** — a non-task line ending with `:` (trailing tags allowed).
- **Note** — any other non-blank line.
- **Tags** — a trailing, whitespace-preceded run of `@name` / `@name(value)`
  at the end of a line. Mid-line `@words` (emails, handles) are plain text.
- **Nesting** — tab indentation nests an item under the item above.

## Features

- **Highlighting** — projects as titles, notes dimmed, tags and values
  distinctly colored.
- **`@done` / `@cancelled` fading** — tagged items and their entire subtree
  (up to 6 levels deep) fade into the theme's ghost style, distinct from
  note dimming. The state tag stays legible: `@done` muted blue,
  `@cancelled` muted amber. Task bullets act as pseudo-checkboxes: accent
  colored while open, faded when closed.
- **Outline** — projects (only) appear in the outline panel and
  `cmd-shift-o`, nested as in the document, shown by bare name.
- **Folding** — indentation-based folding works out of the box.

## Install

Not yet on the Zed extension registry:

1. Clone this repo.
2. In Zed: `zed: extensions` → `Install Dev Extension` → select the clone.

## Marking items `@done` with `alt-d`

Zed extensions cannot add editor commands, so a true in-buffer toggle is
not possible; this binding appends ` @done` to the current line
(`zed: open keymap`):

```json
{
  "context": "Editor && extension == taskpaper",
  "bindings": {
    "alt-d": ["workspace::SendKeystrokes", "end space @ d o n e"]
  }
}
```

## Filtering

For finding tagged items use buffer search (`cmd-f`) or project search
(`cmd-shift-f`) for `@due`, `@due(2026-07`, etc. TaskPaper 3's query
language (date comparisons and the like) is beyond what a Zed extension
can provide.

## Limitations

- Faded, not struck through: Zed theme syntax styles have no strikethrough.
- The fade cascade stops 6 levels below the tagged item.
- Tag values cannot contain `)` or newlines; nested parentheses make the
  run plain text.
- Tags are recognized only at the end of a line (stricter than TaskPaper 3,
  by design).

## Development

See [MAINTAINER.md](MAINTAINER.md). MIT licensed.
