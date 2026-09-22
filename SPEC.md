# alfred-emoji-rs — Design Spec

Rust rewrite of `alfred-emoji-go` + `emoji-go`: an Alfred 5 script filter that
resolves a typed query to emoji, with fuzzy and alias matching that beats a
plain Levenshtein-distance scan.

## 1. What the current Go version does (baseline)

- `emoji-go` flattens every emoji's `AlternateNames` (from iamcal/emoji-data
  short names only) into one big `[]string` of keyword→index pairs, then runs
  `fuzzy.RankFindNormalizedFold` (Levenshtein distance) over that flat list on
  every keystroke, sorts by distance, and maps ranked keywords back to emoji.
- `alfred-emoji-go` wraps that in an AwGo script filter: skin-tone modifier
  from an env var, paste-by-default / ⌘-copy-symbol / ⌥-copy-shortcode, and a
  self-update job. Images are pre-rendered PNGs shipped in the workflow
  bundle, one per `Unified` codepoint (+ skin variant).

Two weaknesses worth fixing:
1. **Alias coverage is thin.** iamcal/emoji-data's `short_name`/`short_names`
   are the *official* CLDR-derived names — good but narrow. There's no
   coverage for the colloquial terms people actually type (":shit:", "poop",
   "facepalm" vs. the canonical "face_palm", etc.).
2. **Levenshtein distance ranks poorly for short-token search.** Edit
   distance treats `"rocket"` vs `"rock"` and `"rocket"` vs `"trceko"`
   (transposed) as similar-ish costs, and it has no concept of prefix,
   word-boundary, or consecutive-run bonuses — the things that make fzf/cmd-T
   style matching feel right for keyword search. It's the wrong scoring
   function for this problem even though it's a fine crate for what it is.

## 2. Data sources (merge three, keep the one already working)

| Source | Gives us | Verdict |
|---|---|---|
| **iamcal/emoji-data** (already vendored via `emojigen`) | Canonical short names, categories, unified codepoints, skin-tone variation map, per-vendor image sheets — this is what currently drives image rendering | **Keep.** Don't touch the image pipeline; it works and re-deriving it from another dataset buys nothing. |
| **github/gemoji** (`db/emoji.json`) | GitHub/Slack-style shortcodes and `tags` arrays, curated and stable, MIT-ish license, small (~2k entries) | **Add.** This is what a Rust crate like `emojis` (docs.rs) already wraps for O(1) shortcode lookup — same idea, but we need our own merge because we still need iamcal's image/skin-tone data, which `emojis` doesn't carry. |
| **muan/emojilib** | Crowdsourced, much broader keyword lists per emoji (e.g. "shit" → 💩 alongside "poop", "hankey") | **Add, filtered.** It's colloquial and occasionally noisy/duplicated — dedupe against gemoji/iamcal terms and cap keywords per emoji (see §4) so common short queries don't get flooded by low-value synonyms. |

Emoji version parity: pin all three to the same Unicode/emoji release the
image sheets support (currently emoji v14, matching `emoji-go`'s pin) so
codepoints line up 1:1 during the merge; a codepoint present in one dataset
but not another for that revision is simply skipped for that source.

Everything is fetched and merged at **build/generate time**, not at runtime —
same shape as `emojigen` today, just reading three sources instead of one and
writing a richer struct.

## 3. Data model

```rust
pub struct Emoji {
    pub name: &'static str,            // canonical short name (iamcal)
    pub category: &'static str,
    pub unified: &'static str,         // hyphenated codepoints
    pub character: &'static str,
    pub shortcodes: &'static [&'static str],  // gemoji aliases, incl. iamcal short_names
    pub keywords: &'static [&'static str],    // gemoji tags + filtered emojilib terms
    pub skin_variations: &'static [(SkinTone, ImageData)],
    pub image: ImageData,
}
```

Two search fields, not one flat list: **shortcodes** (short, high-precision,
colon-wrapped identifiers) and **keywords** (longer tail, lower precision).
They get different score weights (§4) instead of being pre-flattened into a
single undifferentiated term list like the Go version does.

Emit this as a `phf::Map` (or a generated `static` array + `phf_map!` index)
in a build-generated `.rs` file, same spirit as `data.go` today. `phf` gets
you a perfect-hash O(1) exact lookup for the `:shortcode:` fast path and zero
runtime parsing/allocation cost at process start — this matters because a
script filter's whole process lifetime is one query.

## 4. Search algorithm

Replace Levenshtein ranking with **fzf-equivalent scoring** via the
[`nucleo`](https://github.com/helix-editor/nucleo) crate (Helix editor's
matcher — a from-scratch Smith-Waterman implementation, same scoring model as
fzf, ~6x faster than `fuzzy-matcher`/skim in their own benchmarks, and
handles Unicode grapheme boundaries correctly, which matters once emoji
*names* themselves are being matched against). This is a straight upgrade
over `lithammer/fuzzysearch`'s edit-distance approach for short-token
keyword search — prefix hits, word-boundary hits, and consecutive-run hits
all score higher there, which is exactly the intuition users have when they
type "face_p" expecting `face_palm` before some Levenshtein-adjacent noise
word.

Per query:

1. **Exact shortcode hit** (`:rocket:` or `rocket` matching a shortcode
   exactly, case-insensitive): short-circuit, return that emoji first via
   the `phf` map, distance/score irrelevant. This is the ":shortcode:"
   power-user path and should be instant and unambiguous.
2. **Fuzzy pass over shortcodes**, weight **A**.
3. **Fuzzy pass over keywords**, weight **B < A** (so a keyword hit never
   outranks a shortcode hit of similar match quality — shortcodes are curated
   and precise, keywords are long-tail and noisier).
4. Merge per-emoji: an emoji can match via multiple terms (shortcode *and*
   keyword, or several keywords) — take the **max** score across all of an
   emoji's matched terms, not a sum, so an emoji with 20 loosely-related
   keywords doesn't out-rank one with a single tight match.
5. Sort descending by merged score; stable tie-break by `name` alphabetical.
6. Cap keywords-per-emoji at build time (e.g. top ~12 by source priority:
   iamcal short names + gemoji tags first, emojilib fill-in after, dedup
   case-insensitively) so pass 3 doesn't become a haystack search over
   thousands of near-duplicate synonyms per emoji.

This is a two-field weighted merge, not a single flat `RankFind` — the main
structural change from the Go version, and it's what fixes both weaknesses
in §1 at once (better alias coverage *and* better ranking of that coverage).

## 5. Ranking refinement: frecency (stretch, v1.1)

Alfred already does its own usage-based reordering via `UID()` unless you
suppress it (`SuppressUIDs`) — the Go version keeps UIDs on (`info.Name`) for
normal results, which means Alfred's own "knowledge" already re-orders
frequently-chosen emoji upward across invocations. Keep that behavior as-is
rather than reinventing a frecency store; it's simpler and Alfred's ordering
is exactly the desired outcome (recently/frequently picked emoji surface
faster over time). Don't build a custom usage-tracking DB for v1.

## 6. Did embeddings make sense here? (No — explain why, briefly)

Considered and rejected for v1: a semantic embedding index (per-emoji vector
from name+keywords, cosine-similarity search against a query vector) would
help with queries like "feeling adventurous" that share no tokens with any
alias. But it requires either bundling a local embedding model (binary size,
cold-start latency — bad fit for a script filter invoked on every keystroke
and expected to respond in well under 100ms) or a network call (unacceptable
for an offline keyboard-driven tool, and a latency/privacy regression versus
today). The alias-coverage fix in §2 (adding gemoji tags + filtered emojilib
keywords) captures the overwhelming majority of the real gap — people mostly
type synonyms/colloquialisms, not full descriptive phrases. Worth revisiting
only if user feedback after shipping v1 shows a specific class of
"semantic, not lexical" misses that alias expansion doesn't cover.

## 7. Workflow behavior (unchanged from Go version)

- `skin_tone` env var selects the skin-tone variant image/character, same
  five values as today.
- Default action: paste character into frontmost app.
- ⌘: copy character to clipboard.
- ⌥: copy `:shortcode:` to clipboard.
- Empty query: no results (or show update-available notice, if self-update
  is kept — decide separately whether to port AwGo's GitHub-release updater
  or drop it; not a search-design question).
- Images: keep shipping pre-rendered PNGs per `Unified` (+ skin variant),
  looked up by path exactly as today.

## 8. Performance target

Corpus is small (~1,900 emoji, low thousands of shortcode+keyword terms
after dedup/cap). Target: full query → ranked, deduped results in well under
20ms on typical hardware, so the whole process (spawn → parse dataset via
`phf` → match → print Alfred JSON → exit) stays comfortably inside Alfred's
per-keystroke script filter budget. Verify with a `criterion` benchmark over
the merged dataset rather than assuming — `nucleo`'s speed advantage is
mostly relevant at much larger corpora, so confirm it's not overkill for
this dataset size before locking in the dependency.

## 9. Open questions to settle before implementation

- Keep AwGo-equivalent Rust crate (e.g. `alfred` / `powerpack`) or write the
  minimal Alfred JSON output by hand? Given how small the actual surface
  used today is (items, subtitle, icon, arg, var, valid), hand-rolling a
  tiny serde struct may be less dependency weight than porting AwGo's full
  feature set (workflow dirs, magic actions, updater) — worth a quick
  scoping pass against what's actually used in `main.go` before deciding.
- Self-update: port AwGo's GitHub-release updater, or drop self-update from
  the workflow and rely on manual reinstall / Alfred gallery distribution?
- License compatibility check for vendoring emojilib's keyword list
  alongside gemoji/iamcal data in the generated dataset file (all three are
  MIT, but confirm at generate-script-write time, not deferred).

Sources consulted: [helix-editor/nucleo](https://github.com/helix-editor/nucleo),
[iamcal/emoji-data](https://github.com/iamcal/emoji-data),
[github/gemoji](https://github.com/github/gemoji/blob/master/db/emoji.json),
[muan/emojilib](https://github.com/muan/emojilib),
[emojis crate](https://docs.rs/emojis) (precedent for gemoji-backed O(1)
shortcode lookup in Rust).
