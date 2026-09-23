//! `xtask render-icons`: rasterize `images/<unified>.png` for every emoji
//! (and skin-tone variant) from the locally installed Apple Color Emoji
//! font, via the small CoreText helper in `native/render_emoji.c`.
//!
//! This replaces depending on iamcal/emoji-data's vendor sprite sheets
//! (see PLAN.md §0 — those lag a fresh Unicode release by months). A
//! rebuild after a macOS update with new emoji-font support picks up new
//! glyphs automatically; anything the current font can't render yet is
//! skipped (no PNG) rather than shipping a broken image, so the emoji
//! stays searchable/pasteable-by-text even before it has an icon.
//!
//! `images/` is gitignored — same as the Go version — since it's a
//! packaging-time asset regenerated per release, not source.

use std::collections::HashSet;
use std::os::raw::{c_double, c_int};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use emoji_data::EMOJIS;

use crate::emoji_render::{render, RenderOutcome};

const IMAGES_DIR: &str = "images";
const CANVAS_PX: c_int = 144;
const POINT_SIZE: c_double = 118.0;

pub fn run() -> Result<()> {
    let macos_version = Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    eprintln!("rendering icons using the Apple Color Emoji font on macOS {macos_version}");

    std::fs::create_dir_all(IMAGES_DIR).context("creating images/")?;

    let mut targets: Vec<(&str, &str)> = Vec::new(); // (unified, character)
    for emoji in EMOJIS {
        targets.push((emoji.image.unified, emoji.image.character));
        for (_, variant) in emoji.skin_variations {
            targets.push((variant.unified, variant.character));
        }
    }

    let mut seen = HashSet::new();
    let mut written = 0u32;
    let mut unsupported = Vec::new();
    let mut errors = Vec::new();

    for (unified, character) in targets {
        if !seen.insert(unified) {
            continue;
        }
        let out_path = Path::new(IMAGES_DIR).join(format!("{unified}.png"));
        match render(character, POINT_SIZE, CANVAS_PX, &out_path) {
            Ok(RenderOutcome::Written) => written += 1,
            Ok(RenderOutcome::Unsupported) => unsupported.push(unified.to_string()),
            Err(e) => errors.push(format!("{unified}: {e}")),
        }
    }

    eprintln!(
        "wrote {written} icons, {} unsupported by the current font, {} errors",
        unsupported.len(),
        errors.len()
    );
    if !unsupported.is_empty() {
        eprintln!(
            "unsupported (no icon shipped, will fall back to Alfred's default): {}",
            unsupported.join(", ")
        );
    }
    for e in &errors {
        eprintln!("error: {e}");
    }
    if !errors.is_empty() {
        anyhow::bail!("{} icon(s) failed to render", errors.len());
    }
    Ok(())
}
