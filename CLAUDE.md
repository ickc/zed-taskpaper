# CLAUDE.md

Zed extension for TaskPaper: tree-sitter grammar + queries, plus the
taskpaper-ls language server (crates/taskpaper-ls) and its WASM launcher
glue (src/lib.rs). Read MAINTAINER.md for architecture and workflows; it is
the source of truth.

Hard rules:

- Three places encode the TaskPaper line rules and must stay in sync:
  tree-sitter-taskpaper/grammar.js + src/scanner.c, and
  crates/taskpaper-ls/src/model.rs.
- Releases MUST be cut by pushing a v* tag (CI attaches taskpaper-ls
  binaries; the extension downloads the latest release's assets — a
  release without assets breaks installs).

- After any change to `tree-sitter-taskpaper/grammar.js` or `src/scanner.c`:
  run `pixi run generate`, make `pixi run test` pass, commit the regenerated
  `src/`, push, then update `[grammars.taskpaper] commit` in `extension.toml`
  to the new SHA and push again. Query/doc changes need no re-pin.
- Keep `is_tag_name_char()` (scanner.c) and the `tag_name` regex (grammar.js)
  in sync, and keep the scanner's backward tag-stripping consistent with the
  grammar's forward tag parsing — mismatches produce ERROR nodes.
- Never create a `grammars/` directory (Zed claims that path at install
  time); it is gitignored.
- In `highlights.scm`, pattern order is load-bearing: base styles, then the
  done/cancelled wash, then state-tag accents last.
- Zed caps highlight-query matching at 64 concurrent matches (the
  tree-sitter CLI does not, so `tree-sitter query` passing proves nothing
  about Zed). Subtree fading therefore lives in the scanner (dim_* nodes),
  NOT in deep wildcard query patterns — never move it back.
- Verify with `pixi run ci` before pushing; CI must stay green on Linux and
  macOS.
