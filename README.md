# alfred-emoji-rs

An [Alfred 5](https://www.alfredapp.com) workflow for looking up and
inserting emoji by keyword — fuzzy matching plus a broad alias/keyword
index, so `:rocket:`, `rocket`, `roc`, and even loosely-related words like
`launch` all find 🚀.

## Features

- **Keyword search**: type `e` in Alfred followed by a name, alias, or
  related word.
- **Fuzzy matching**: typos and partial words still find the right emoji,
  ranked with exact/prefix matches first.
- **Three actions per result**:
  - ↩ — paste the emoji into the frontmost app.
  - ⌘↩ — copy the emoji character to the clipboard.
  - ⌥↩ — copy its `:shortcode:` to the clipboard.
- **Skin tone support** for emoji that have skin-tone variants, via a
  workflow variable.

## Requirements

- macOS with [Alfred 5](https://www.alfredapp.com) (Powerpack required
  for script filters).
- To build the workflow yourself: a Rust toolchain (see `mise.toml`) and
  Xcode Command Line Tools (`xcode-select --install`) for the icon
  renderer, which links against the system's CoreText/CoreGraphics
  frameworks.

## Building and installing the workflow

The dataset (`crates/emoji-data/src/generated.rs`) is checked in, so you
don't need to regenerate it to build the workflow — only the icons and
the packaged bundle need to be produced locally, since both depend on
the machine they're built on (the installed emoji font, and a release
binary for your Mac).

```bash
# 1. Rasterize emoji icons from your Mac's installed emoji font.
cargo run -p xtask -- render-icons

# 2. Build the release binary and zip everything into a .alfredworkflow.
cargo run -p xtask -- package
```

This produces `dist/alfred-emoji-rs.alfredworkflow`. Open it —

```bash
open dist/alfred-emoji-rs.alfredworkflow
```

— and Alfred will prompt you to import the workflow. Confirm, and you're
done.

If you ever want to pick up newer emoji or aliases (a new Unicode
release, updated gemoji/emojilib data, or a macOS update that adds emoji
you didn't have icons for before), see the `refresh-emoji-data` skill in
this repo, or just re-run `cargo run -p xtask -- generate` before the two
steps above.

## Usage

Type the workflow keyword (`e` by default) followed by your search term:

```
e rocket
e face palm
e :thumbsup:
```

### Skin tones

Open the workflow's configuration in Alfred Preferences and set the
`skin_tone` variable to one of: (empty, for the default tone), `light`,
`medium_light`, `medium`, `medium_dark`, or `dark`. It applies to every
emoji that supports skin-tone variants.
