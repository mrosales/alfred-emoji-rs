# alfred-emoji-rs

Alfred 5 script filter for emoji search — fuzzy + alias matching over a
merged Unicode/gemoji/emojilib dataset, targeting Emoji 18.0.

Rust rewrite of [alfred-emoji-go](https://github.com/mrosales/alfred-emoji-go)
and [emoji-go](https://github.com/mrosales/emoji-go).

See [SPEC.md](SPEC.md) for the design and [PLAN.md](PLAN.md) for the
implementation plan and current phase status.

## Workspace layout

- `crates/emoji-data` — the emoji dataset (generated) and search index.
- `crates/alfred-emoji` — the script filter binary Alfred invokes.
- `xtask` — dataset generation (`generate`) and icon rendering
  (`render-icons`) tasks; run via `cargo run -p xtask -- <command>`.

## Status

Phase 1 (scaffolding) and Phase 2 (dataset generation) done. Run
`cargo run -p xtask -- generate` to refresh `crates/emoji-data/src/generated.rs`
from Unicode/gemoji/emojilib; the generated file is committed so normal
builds don't need network access. No search or Alfred output yet
(Phase 3+).
