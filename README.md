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

Phases 1–6 done. Manual in-Alfred validation is still open (Phase 7).

- `cargo run -p xtask -- package` builds the release binary and zips it
  with `info.plist` and `images/` into `dist/alfred-emoji-rs.alfredworkflow`
  (run `generate`/`render-icons` first — package doesn't do that for you).
  Idiomatic-Rust choice per PLAN.md: this is a `cargo-xtask` subcommand,
  not a `justfile`/`mage` task, since the project already has an `xtask`
  crate and cargo-xtask is the community-standard way to do build
  automation without another tool on `$PATH`.

- `emoji_data::search(query)` does exact-shortcode lookup plus weighted
  `nucleo` fuzzy search over shortcodes/keywords (SPEC.md §4). Benchmarked
  at ~150µs/query over the full dataset in release mode — see
  `crates/emoji-data/benches/search.rs`.
- `alfred-emoji <query>` prints Alfred script filter JSON: paste by
  default, ⌘ copies the character, ⌥ copies `:shortcode:`, `skin_tone`
  env var selects the variant. Run it from the workspace root (or
  wherever `images/` lives) so icon paths resolve. Try:
  `cargo run -p alfred-emoji -- rocket`.

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
