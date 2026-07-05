# zed-taskpaper

[TaskPaper](https://www.taskpaper.com) language support for
[Zed](https://zed.dev): syntax highlighting, tag-aware styling, and outline
navigation for plain-text todo lists.

It is a pure language extension — a tree-sitter grammar (included in this
repo under `grammars/taskpaper`) plus queries. No language server, no
background processes.

## The TaskPaper format

The grammar follows the [TaskPaper 3 conventions](https://guide.taskpaper.com/getting-started/):

- **Task** — a line starting with `- `.
- **Project** — a non-task line ending with `:` (trailing `@tags` after the
  colon are allowed).
- **Note** — any other non-blank line.
- **Tags** — `@name` or `@name(value)` anywhere in a line.
- **Nesting** — indentation with tabs (spaces are tolerated) makes an item a
  child of the item above.

## Features

- **Highlighting** — projects styled as titles, notes dimmed, tasks plain,
  tags and tag values distinctly colored.
- **`@done` fading** — the whole line of an item tagged `@done` is faded
  like a comment. (Zed theme syntax styles have no strikethrough, so fading
  is the closest sensible rendering.)
- **Outline panel** — projects and tasks appear in the outline panel
  (`outline panel: toggle focus`), nested exactly like the document. Tags
  are shown as dimmed context next to each item.
- **Folding** — indentation-based folding of projects/subtrees works out of
  the box.
- **Breadcrumbs & symbol search** — `cmd-shift-o` fuzzy-searches projects
  and tasks in the current file.

## Filtering (without losing context)

TaskPaper 3 has a query language for filtering; a Zed extension cannot add
one (extensions cannot create panels or virtual buffers). The outline panel
gets you most of the way there:

1. Open the outline panel and focus its filter box.
2. Type e.g. `@due`, `@today`, or any text.

Because tags are part of each outline entry, tag filters work, and the
panel keeps ancestor items visible for every match — so a matching task is
always shown inside its project chain, and clicking it jumps to the real
location in the buffer. For ad-hoc filtering across files, project search
(`cmd-shift-f`) for `@due(2026-07` etc. also works well.

Date-comparison queries ("due on or before today") are out of scope for an
extension; if you need them, pair the file with the TaskPaper app or a CLI.

## Marking items `@done` quickly

Extensions cannot register new editor actions, but Zed's `SendKeystrokes`
macro covers the common case. Add to your `keymap.json` (`zed: open keymap`):

```json
[
  {
    "context": "Editor && extension == taskpaper",
    "bindings": {
      "alt-d": ["workspace::SendKeystrokes", "end space @ d o n e"]
    }
  }
]
```

This appends ` @done` to the current line. Pick any binding you like
(`cmd-d` is taken by Zed's multi-cursor selection by default).

## Installing

Not yet on the Zed extension registry. To install as a dev extension:

1. Clone this repo.
2. In Zed: `zed: extensions` → `Install Dev Extension` → select the clone.

Zed fetches and compiles the grammar at the commit pinned in
`extension.toml`, so the clone itself only provides the manifest and
queries.

## Development

Uses [pixi](https://pixi.sh) for the toolchain (tree-sitter CLI + C
compiler):

```sh
pixi run generate   # regenerate src/parser.c from grammar.js
pixi run test       # corpus tests (grammars/taskpaper/test/corpus)
pixi run parse      # parse examples/demo.taskpaper and print the tree
pixi run ci         # what CI runs: generate-check + test
```

The generated parser (`grammars/taskpaper/src/`) is committed because Zed
builds the grammar from the repo as-is. After changing the grammar, run
`pixi run generate`, commit, and update `commit` in `extension.toml` to the
new SHA.

## Known limitations

- `@word` mid-token (e.g. `user@example.com`) is highlighted as a tag; the
  official apps require a word boundary. Harmless, purely cosmetic.
- Tag values cannot contain `)` or newlines.
- Lines longer than 4096 characters are classified by prefix only (a
  colossal line ending in `:` becomes a note).

## License

MIT
