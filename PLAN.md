# Implementation Plan — alfred-emoji-rs (targeting Emoji 18.0)

## 0. A wrinkle that changes the plan: Emoji 18.0 just shipped

Emoji 18.0 released 2026-09-16 (five days before this plan), alongside
Unicode 18.0. That timing breaks the SPEC's original assumption that we can
just vendor iamcal/emoji-data's sprite sheets for images:

- `iamcal/emoji-data` supports Emoji 17.0 as of its last known release, and
  its own PRs note that Unicode-version bumps wait on **vendor** (Apple/
  Google/etc.) artwork before merging — vendor emoji-font updates for a
  September Unicode release typically land with the following spring's OS
  update. Emoji 18 glyphs in iamcal's sheets are likely **months** away.
- `gemoji` and `emojilib` (our alias/keyword sources) will lag similarly —
  they're maintained by hand and don't track a Unicode release day-of.

Consequence: if "latest emoji set" means the new Emoji 18 characters need to
be searchable and pasteable *now*, we can't wait on any of the three
GitHub-hosted datasets for those ~19 new codepoints. Two things change from
the SPEC:

1. **Codepoint/name source of truth becomes Unicode's own
   `emoji-test.txt` for v18** (published same-day at unicode.org), not
   iamcal. It's the only source guaranteed current. iamcal/gemoji/emojilib
   become *keyword enrichment* layers we merge on top where they have
   coverage, not the source of the emoji list itself.
2. **Icons stop depending on vendor sprite sheets and get rendered locally
   from the installed system emoji font instead.** This workflow only ever
   runs on macOS under Alfred, so we're not giving up cross-platform
   compatibility we ever had — we're trading a stale third-party sprite
   sheet for the same Apple Color Emoji font macOS already renders emoji
   with everywhere else. It also means: the day Apple ships an OS update
   with Emoji 18 glyphs, a rebuild picks them up with zero dataset-vendor
   dependency. Until then, unrendered codepoints get a documented fallback
   (§4) instead of silently missing from the workflow.

This is the main design delta from SPEC.md. Everything else in the SPEC
(nucleo-based weighted shortcode/keyword search, phf-backed static table,
paste/copy/copy-shortcode actions) stands unchanged.

## 1. Phased plan

### Phase 1 — Repo scaffolding
- `cargo init` binary crate `alfred-emoji`, workspace-style split if useful:
  `alfred-emoji` (binary/script-filter logic) + `emoji-data` (library: the
  dataset, `Emoji` struct, search index) — mirrors the Go repo split
  (`alfred-emoji-go` ↔ `emoji-go`) but as one Cargo workspace instead of two
  repos, since there's no reason for the library to be independently
  versioned right now.
- `mise`/toolchain pin, `LICENSE` (MIT, matching both source repos), CI
  stub (`cargo test`, `cargo clippy`) — no need for more CI than that yet.

### Phase 2 — Dataset generator (`xtask` or `build.rs`)
Decision: use a separate `cargo xtask generate` binary rather than doing
this in `build.rs`. Fetching from the network and rasterizing fonts on
*every* `cargo build` is slow and flaky; this should be a deliberate,
committed step (generated `.rs` file checked into the repo, regenerated
on demand when datasets are refreshed), same operational shape as
`emojigen`/`go generate` in the Go version today.

Steps the generator performs:
1. Download/parse `emoji-test.txt` (Unicode v18) → canonical list of
   codepoints, status (`fully-qualified` only), group/subgroup, and
   Unicode's own short description string.
2. Download/parse `gemoji/db/emoji.json` → for codepoints it covers,
   attach `aliases` (shortcodes) and `tags` (keywords).
3. Download/parse `emojilib`'s `emojis.json` → for codepoints it covers,
   attach additional keywords, filtered: lowercase, dedupe against
   gemoji tags case-insensitively, cap at ~12 total keywords/emoji
   (priority order: gemoji aliases > gemoji tags > emojilib fill-in),
   per SPEC §4.
4. For any Emoji 18 codepoint with **no** gemoji/emojilib coverage yet:
   fall back to a slugified version of Unicode's own description string
   as both the sole shortcode and sole keyword (e.g. "flag: Sark" →
   `flag_sark`). Not as good as curated aliases, but keeps the emoji
   findable by its actual name from day one instead of being invisible
   until upstream catches up.
5. Emit `crates/emoji-data/src/generated.rs`: a `&'static [Emoji]` array
   plus a `phf::Map<&'static str, u16>` (shortcode → index into the array)
   for the O(1) exact-match fast path.
6. Record dataset provenance (source commit SHAs/fetch date per source) in
   a comment header of the generated file, so a future re-run's diff is
   legible and "why did this emoji's keywords change" is answerable.

Acceptance: generator run produces a file that compiles, contains all
Emoji 18 codepoints from `emoji-test.txt`, and every entry has at least one
shortcode and one keyword (even if it's the fallback slug from step 4).

### Phase 3 — Icon rendering pipeline
Replaces "copy PNGs from iamcal's sheets" with "rasterize from the local
system font," run as its own `xtask render-icons` step (separate from
Phase 2 since it needs to run on macOS specifically, and dataset generation
doesn't).

1. For each `Emoji` (and each skin-tone variant), rasterize the character
   using Core Text against the system emoji font at a fixed point size,
   to a transparent-background PNG, same file-naming convention as today
   (`images/<unified>.png`) so the rest of the pipeline (icon lookup by
   `Unified` string in the script filter) doesn't change.
2. **Detect unrenderable glyphs**: query `CTFontGetGlyphsForCharacters`
   (or equivalent) for glyph coverage before rasterizing; if the font
   reports no glyph (or renders `.notdef`/tofu), don't ship a broken PNG —
   fall back to Alfred's default icon (i.e., emit no custom `icon` key for
   that item) so the emoji is still searchable/pasteable by text even
   before the OS font supports it visually.
3. This step is inherently tied to whatever macOS version does the
   rendering — document that in the README (which macOS/font version the
   shipped icons were generated on) so re-running the render step after an
   OS update is understood as the mechanism for picking up new glyphs, not
   a dataset re-fetch.

Acceptance: `images/` directory populated for every codepoint the local
font can render; a logged list of codepoints that fell back to no-icon
(expected to include some/most Emoji 18 additions until Apple ships
support).

### Phase 4 — Search engine (`emoji-data` crate)
As specified in SPEC.md §4, implemented now against the real generated
dataset:
- `nucleo`-based matcher, two passes (shortcodes weight A, keywords weight
  B<A), max-score merge per emoji, exact-shortcode fast path via the `phf`
  map before falling to fuzzy matching at all.
- Unit tests: exact shortcode match, prefix match ranks above scattered
  fuzzy match, an emoji with one tight keyword match outranks one with many
  loose matches (guards the "max not sum" merge rule), empty query
  behavior, and at least one Emoji-18-specific fixture (search by the
  fallback slug from Phase 2 step 4 resolves correctly).
- `criterion` benchmark over the full merged corpus, confirming the
  sub-20ms target from SPEC.md §8 before locking in `nucleo` as a real
  dependency rather than assumed-fine.

### Phase 5 — Script filter binary
- Minimal hand-rolled Alfred JSON output via `serde`/`serde_json` (decision
  from SPEC.md's open question: skip porting AwGo's full feature set —
  scoping `main.go` shows only items/subtitle/icon/arg/var/valid/UID are
  actually used, which is a small enough surface to own directly and avoid
  a heavyweight dependency).
- Skin-tone env var, paste/copy/copy-shortcode actions, `UID` kept per
  emoji name so Alfred's own knowledge-based reordering still works
  (SPEC.md §5 — no custom frecency store).
- No self-update in v1 (decision: drop AwGo's GitHub-release updater;
  distribute via Alfred gallery or manual `.alfredworkflow` releases
  instead — revisit only if that proves annoying in practice).

### Phase 6 — Packaging
- `info.plist` mirroring the existing Alfred object graph (script filter →
  conditional on `{var:action}` → clipboard output, paste vs. copy), keyword
  `e`, `skin_tone` variable — same UX contract as today so muscle memory
  transfers.
- Bundle `images/` output from Phase 3, the compiled binary, `info.plist`
  into a `.alfredworkflow` zip; a `just`/`mage`-equivalent task for this
  (the Go version used `mage.go` — port the same task names if useful for
  muscle memory, otherwise a plain `justfile` is fine).

### Phase 7 — Validation
- Manual test pass in real Alfred: keyword `e`, a handful of exact
  shortcodes, a handful of fuzzy/typo queries, one Emoji-18 fallback-slug
  query, skin-tone variable set to each of the five values, all three
  actions (paste/copy/copy-shortcode).
- Confirm the no-icon fallback (Phase 3.2) renders reasonably in Alfred's
  UI (default icon, not a broken image glyph).

## 2. Open items still worth a decision before Phase 5

- Workspace layout (`emoji-data` lib + `alfred-emoji` bin in one repo) —
  proceeding with this as the default; say so if you'd rather mirror the
  Go version's two-repo split.
- Whether "Emoji 18 support" for this v1 means *searchable-with-fallback*
  (as planned above) or you want to hold the release until iamcal/gemoji
  catch up with real curated aliases — current plan assumes the former
  (ship now, fallback slugs are fine short-term) since it's a small,
  self-correcting gap that a dataset regen fixes later for free.
