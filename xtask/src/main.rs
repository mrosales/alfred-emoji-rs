//! Task runner: `cargo run -p xtask -- <command>`.
//!
//! - `generate`: build `crates/emoji-data/src/generated.rs` from
//!   emoji-test.txt + gemoji + emojilib (Phase 2).
//! - `render-icons`: rasterize `images/*.png` from the system emoji font
//!   (Phase 3, macOS only — see render_icons.rs for why).
//! - `icon`: rasterize the workflow's own `icon.png` (macOS only — see
//!   icon.rs).
//! - `package [--sign]`: build the release binary and zip it with
//!   info.plist and images/ into a .alfredworkflow bundle (Phase 6).
//!   `--sign` code-signs and notarizes it first — see package.rs.
//!
//! Must be run from the workspace root (paths are relative to it).

mod build;
mod generate;
mod package;

#[cfg(target_os = "macos")]
mod emoji_render;

#[cfg(target_os = "macos")]
mod render_icons;

#[cfg(not(target_os = "macos"))]
mod render_icons {
    pub fn run() -> anyhow::Result<()> {
        anyhow::bail!("render-icons uses CoreText and only runs on macOS")
    }
}

#[cfg(target_os = "macos")]
mod icon;

#[cfg(not(target_os = "macos"))]
mod icon {
    pub fn run() -> anyhow::Result<()> {
        anyhow::bail!("icon uses CoreText and only runs on macOS")
    }
}

#[cfg(target_os = "macos")]
mod notarize;

#[cfg(not(target_os = "macos"))]
mod notarize {
    use std::path::Path;
    pub fn sign_and_notarize(_binary: &Path) -> anyhow::Result<()> {
        anyhow::bail!("notarize uses codesign/notarytool and only runs on macOS")
    }
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let command = args.next();
    match command.as_deref() {
        Some("generate") => generate::run(),
        Some("render-icons") => render_icons::run(),
        Some("icon") => icon::run(),
        Some("package") => {
            let sign = args.next().as_deref() == Some("--sign");
            package::run(sign)
        }
        _ => {
            eprintln!("usage: cargo run -p xtask -- <generate|render-icons|icon|package [--sign]>");
            std::process::exit(2);
        }
    }
}
