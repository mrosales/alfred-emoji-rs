---
name: refresh-emoji-data
description: Refresh alfred-emoji-rs's generated dataset, rendered icons, and packaged .alfredworkflow after an upstream change — a new Unicode/Emoji release, a gemoji or emojilib update, or a macOS update that adds emoji-font glyph support. Use when asked to refresh/update/regenerate the emoji dataset, icons, or workflow package, to check for a new Unicode/Emoji version, or to pick up newly-supported glyphs after an OS update.
---

# Refresh emoji data

This project's dataset (`crates/emoji-data/src/generated.rs`) and icons
(`images/`, not committed) are both point-in-time snapshots produced by
`xtask` from sources that move independently of this repo: Unicode's
`emoji-test.txt`, gemoji, emojilib, and whatever emoji font macOS ships.
See `PLAN.md` §0 and `README.md` for why each of those is regenerated
rather than hand-maintained.

This skill is the documented procedure for bringing those snapshots back
in sync — run it manually on a cadence that matches how the underlying
sources actually change (roughly):

- **Unicode/Emoji version bump**: happens about once a year, historically
  around September. Worth checking a few times a year. A bump only ships
  if macOS's emoji font can already render it — see step 3.
- **gemoji / emojilib alias curation**: both are maintained by hand and
  update on no fixed schedule. Worth checking after a new Emoji version
  ships (there's usually a lag before they add curated aliases for it —
  see the "fallback-only" count below) or every few months otherwise.
- **macOS update**: after any macOS update, if the dataset has emoji the
  previous OS couldn't render (check the "unsupported" list from the last
  `render-icons` run, or grep this repo's git log for prior run output),
  it's worth re-running `render-icons` to see if the new OS closed the
  gap — and, if `EMOJI_TEST_URL` (`xtask/src/generate.rs`) is pinned
  below `latest` because of a prior gap, whether it can go back to
  tracking `latest`.

Do not run this speculatively on every session — only when the user asks
for a refresh, or when one of the triggers above is the explicit reason
for the conversation.

## Procedure

Run every step from the workspace root. Report the diff summary from
each step to the user as you go — don't silently batch everything into
one final report, since an unexpected count (e.g. gemoji coverage
*dropping*) partway through is worth surfacing immediately rather than
after also re-rendering 3000+ icons.

### 1. Check whether anything upstream has actually moved

Before regenerating, check what would change:

```bash
curl -s https://unicode.org/Public/emoji/latest/emoji-test.txt | grep '^# Version:'
```

Compare against the `Version:` line in `crates/emoji-data/src/generated.rs`'s
header comment. If they match and the user's trigger was specifically
"is there a new emoji version", you can stop here and report "no change"
— no need to regenerate.

For a macOS-update-triggered refresh, compare `sw_vers -productVersion`
against the macOS version noted in the last `render-icons` run (see
README.md's Status section, or `git log --grep=render-icons -p` for the
commit that last touched icon generation).

### 2. Regenerate the dataset

```bash
cargo run -p xtask -- generate
```

This prints a merge summary (`merged N emoji (X with gemoji coverage, Y
with emojilib coverage, Z fallback-only)`) and any shortcode-collision
warnings — include both in your report. Then:

```bash
git diff --stat crates/emoji-data/src/generated.rs
git diff crates/emoji-data/src/generated.rs | grep -E '^[+-]\s+name:' | sort | uniq -c
```

The second command shows which emoji names were added/removed and which
had their `shortcodes`/`keywords` arrays change. Call out specifically:
newly-added emoji (a real Unicode/Emoji version bump — treat this as
provisional until step 3 confirms macOS can render it), and any
*fallback-only* entries from the previous run that now have real
gemoji/emojilib coverage (upstream curation caught up — name which ones).

### 3. Re-render icons, and confirm any version bump is renderable

```bash
cargo run -p xtask -- render-icons
```

Compare the new `wrote N icons, M unsupported` line and the unsupported
codepoint list against the previous run. If `M` dropped, name which
codepoints gained icon support (the macOS-update payoff). If this is
running on a different macOS version than last time, say so explicitly —
it's the kind of fact that explains *why* coverage changed and belongs in
the commit message (see README.md's existing note on this).

**If step 2 picked up a Unicode/Emoji version bump**, cross-check its
newly-added codepoints (the `+` lines under `image: ImageData { unified:
...` in step 2's diff) against the `unsupported` list this step just
printed.

- **None of them are unsupported** — the installed macOS emoji font
  already covers this Emoji version. Proceed; the dataset stays pointed
  at `emoji/latest/` (`EMOJI_TEST_URL` in `xtask/src/generate.rs`).
- **Any of them are unsupported** — this macOS version can't render this
  Emoji release yet (this is what happened going into Emoji 18.0 on
  2026-09-22: 9 new codepoints had no font glyph). Don't ship a version
  bump macOS can't back: an icon gap for a brand-new emoji is harder to
  tell apart from a broken render than a clean fallback-only entry. Pin
  instead of shipping it:
  1. `git checkout -- crates/emoji-data/src/generated.rs`, then edit
     `EMOJI_TEST_URL` to the last fully-supported version. Unicode
     doesn't always publish a `/Public/emoji/<version>/` archive for a
     brand-new release, but the matching UCD release tree mirrors the
     same file per-version — `curl -sI` it first to confirm it exists
     and its `# Version:` line matches:
     `https://unicode.org/Public/<major>.0.0/emoji/emoji-test.txt`
  2. Redo step 2 and this step against the pinned URL; the diff should
     now be empty (or match only what the pin intends).
  3. State in the report which version got pinned and why, and that
     `EMOJI_TEST_URL` should go back to tracking `latest` once a macOS
     update ships that Emoji version's glyphs (see the macOS-update
     trigger above).

### 4. Verify nothing broke

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All three must be clean before proposing a commit. If a test fails
because a fixture emoji's shortcode/keyword list changed upstream (e.g.
a test hardcodes `"farewell"` as a keyword of "wave" — see
`crates/emoji-data/src/search.rs`), fix the test to match the new
reality rather than forcing the old assertion, and say so in the report.

### 5. Repackage

```bash
cargo run -p xtask -- package
```

Confirms the whole pipeline still produces a valid `.alfredworkflow`
(this also re-runs `cargo build --release`, which is a second compile
check beyond `cargo test`'s debug build).

### 6. Report and stop — don't commit automatically

Summarize for the user:

- Old vs. new Unicode/Emoji version (if changed), and whether it's
  tracking `emoji/latest/` or pinned because macOS doesn't support the
  newer one's glyphs yet (step 3).
- Emoji added/removed, and any fallback-only entries that gained curated
  aliases.
- Icon coverage change (count, and which codepoints if it's a short
  list), and the macOS version used.
- Confirmation that fmt/clippy/test/package all passed.

Only commit if the user asks — this repo's standing instructions require
explicit confirmation before committing (see the git safety protocol),
and a dataset refresh is exactly the kind of change worth letting the
user glance at before it lands, since it can shift search rankings for
existing emoji (new gemoji tags competing with what a user already
expects) as a side effect of picking up new ones.
