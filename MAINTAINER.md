# Maintainer notes

## Layout

| Path | What |
| --- | --- |
| `extension.toml` | Zed extension manifest; pins the grammar to a commit of this repo |
| `languages/taskpaper/` | Zed language config + `highlights.scm` / `outline.scm` |
| `tree-sitter-taskpaper/` | The grammar: `grammar.js`, external scanner, generated parser, corpus tests |
| `examples/demo.taskpaper` | Kitchen-sink example used by `pixi run parse` and manual testing |

Do **not** create a `grammars/` directory: Zed's extension builder checks
the grammar repo out into `<extension>/grammars/<name>` at install time, and
a real directory there collides with the checkout ("failed to compile
grammar"). It is gitignored for exactly this reason.

## Toolchain

Everything runs through [pixi](https://pixi.sh) (tree-sitter CLI + C compiler
from conda-forge):

```sh
pixi run generate   # grammar.js -> src/parser.c (generated sources are committed)
pixi run test       # corpus tests in tree-sitter-taskpaper/test/corpus
pixi run parse      # parse examples/demo.taskpaper, print the tree
pixi run ci         # what CI runs: generate-check + test
```

CI (`.github/workflows/ci.yml`) runs `pixi run ci` on Linux and macOS;
`tree-sitter test` compiles the parser and C scanner natively, so both
platforms exercise the scanner.

## Grammar architecture

The external scanner (`src/scanner.c`) does all the heavy lifting; the
grammar itself is LR(1) with no conflicts:

- **Indent tracking** — python-style INDENT/DEDENT stack; one column per
  character, tabs canonical, spaces tolerated.
- **Line classification** — at each line start the scanner buffers the whole
  line (lookahead only) and emits `_project_begin` / `_task_begin` /
  `_note_begin`; the marker token absorbs leading whitespace and blank lines.
- **Trailing-tag model** — tags only count as a trailing, whitespace-preceded
  run of `@name` / `@name(value)`. `body_end_without_tags()` strips them
  backward from end-of-line during classification, and the resulting body
  length is stashed in scanner state (`pending`) because the lexer cannot
  rewind. The external `text` token later consumes exactly that many
  characters; for projects the terminating `:` is excluded too (it is an
  ordinary internal token, which is what lets the outline show bare names).

Invariant: `is_tag_name_char()` in scanner.c and the `tag_name` regex in
grammar.js must stay in sync (ASCII alnum, `_`, `.`, `-`, any non-ASCII).

## Changing the grammar

Zed compiles the grammar from this repo at the commit pinned in
`extension.toml`, so grammar changes are a two-push dance:

1. Edit `grammar.js` / `scanner.c`; `pixi run generate`; make `pixi run test`
   pass (update/add corpus tests).
2. Commit (including regenerated `src/`) and push.
3. Set `[grammars.taskpaper] commit` to the new SHA, commit, push.

Query/config/README changes don't touch the grammar and need no re-pin.

To reproduce what Zed does at install time (useful when an install fails):
wasi-sdk clang, `-fPIC -shared -Os -Wl,--export=tree_sitter_taskpaper
-I src src/parser.c src/scanner.c`.

## Highlighting notes

- In Zed, later patterns in `highlights.scm` take precedence for the same
  node, and inner captures beat outer ones — which is why the
  `@done`/`@cancelled` "wash" targets leaf nodes (`text`, `marker`,
  `tag_name`, …) rather than whole items, and why the state-tag accent
  patterns sit at the bottom of the file.
- Tree-sitter queries cannot recurse, so the subtree fade is spelled out per
  depth (currently 1–6 levels below the tagged item).
- Style palette: notes `@comment`; done/cancelled wash `@predictive`
  (ghost style); `@done` tag `@hint`; `@cancelled` tag `@string.special`.
  All are defined by Zed's first-party themes.

## Releasing

```sh
git tag vX.Y.Z && git push --tags
gh release create vX.Y.Z --generate-notes
```

Keep `version` in `extension.toml` (and `pixi.toml`, `tree-sitter.json`) in
step with the tag.
