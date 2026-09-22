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

Phases 1–3 done. No search or Alfred output yet (Phase 4+).

- `cargo run -p xtask -- generate` refreshes
  `crates/emoji-data/src/generated.rs` from Unicode/gemoji/emojilib. The
  generated file is committed so normal builds don't need network access.
- `cargo run -p xtask -- render-icons` (macOS only) rasterizes
  `images/<unified>.png` for every emoji + skin-tone variant from the
  locally installed Apple Color Emoji font. `images/` is gitignored — it's
  a packaging-time asset, regenerated per release, same as the Go version.
  Whichever macOS version generates the icons for an actual release should
  be noted in that release's notes, since font glyph coverage (and
  therefore which emoji get a real icon vs. no icon) depends on it —
  the last local run used macOS 27.0 and produced 3,444 icons with 19
  codepoints unsupported by that OS's font (mostly the newest Emoji 18
  additions).
