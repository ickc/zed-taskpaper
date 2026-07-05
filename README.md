# zed-taskpaper

[TaskPaper](https://www.taskpaper.com) language support for [Zed](https://zed.dev):
syntax highlighting, `@done`/`@cancelled` fading, and outline navigation via
a tree-sitter grammar, plus a lightweight language server (`taskpaper-ls`)
for due-date awareness, live task counts, and one-keystroke task workflows.

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
  (any depth) fade into the theme's ghost style, distinct from note
  dimming. The state tag stays legible: `@done` muted blue,
  `@cancelled` muted amber. Task bullets act as pseudo-checkboxes: accent
  colored while open, faded when closed.
- **Outline** — projects (only) appear in the outline panel and
  `cmd-shift-o`, nested as in the document, shown by bare name.
- **Folding** — indentation-based folding works out of the box (a `@done`
  subtree folds like any other indented block).

From the language server (all passive once installed):

- **Due-date diagnostics** — open items with `@due`/`@start` dates in the
  past get a warning ("due (3 days overdue)"); items due today, or tagged
  `@today`, get an info. The project diagnostics panel (`cmd-shift-m`)
  thereby doubles as a "what needs attention" view across all files.
- **Inlay hints** — live task counts after every project heading
  (`Home:  3 open`) and countdowns after future `@due` tags (`in 3 days`).
  Enable inlay hints in settings: `"inlay_hints": {"enabled": true}`.
- **Hover** — hover a project for its count breakdown, or a
  `@due`/`@done`/`@cancelled` tag for relative dates.
- **Code actions** (`cmd-.` on any line):
  - *Mark/Remove `@done`* (stamps today's date) and *`@cancelled`* — the
    toggle, operating on the buffer, unsaved changes and all.
  - *Archive finished items* — moves every finished subtree to a top-level
    `Archive:` project (created on demand, newest at top), stamping each
    with a `@project(Parent / Child)` breadcrumb so the parent chain is
    preserved — the same convention as the TaskPaper 3 app.
  - *Sort children by `@due`* and *task ↔ note conversion*.
- **Tag completion** — type `@` for tag names ranked by workspace usage;
  inside `@due(` get date suggestions (today / tomorrow / next week).
- **Tag rename** — rename a `@tag` across the whole workspace.
- **Project search** — projects across all `.taskpaper` files in the
  open-symbols picker.
- **Formatting** — `editor: format` (or format-on-save) strips trailing
  whitespace, converts leading spaces to tabs, and normalizes tag spacing.

The language server binary is downloaded automatically from this repo's
GitHub releases on first use (or built locally: `pixi run build-lsp`, then
put `taskpaper-ls` on PATH, or set `lsp.taskpaper-ls.binary.path`).

## Install

Not yet on the Zed extension registry:

1. Clone this repo.
2. In Zed: `zed: extensions` → `Install Dev Extension` → select the clone.

## Marking items `@done` with `alt-d`

The *Mark @done* code action is one keystroke away via `cmd-.`. For a
dedicated key, bind the code-action menu (`zed: open keymap`):

```json
{
  "context": "Editor && extension == taskpaper",
  "bindings": {
    "alt-d": "editor::ToggleCodeActions"
  }
}
```

## Filtering

Overdue and due-today items surface automatically in the diagnostics
panel (`cmd-shift-m`), across every file in the project. For ad-hoc tag
filtering use buffer search (`cmd-f`) or project search (`cmd-shift-f`)
for `@due`, `@due(2026-07`, etc.

## Limitations

- Faded, not struck through: Zed theme syntax styles have no strikethrough.
- Tag values cannot contain `)` or newlines; nested parentheses make the
  run plain text.
- Tags are recognized only at the end of a line (stricter than TaskPaper 3,
  by design).

## Development

See [MAINTAINER.md](MAINTAINER.md). MIT licensed.
