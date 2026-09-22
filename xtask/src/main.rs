//! Task runner: `cargo run -p xtask -- <command>`.
//!
//! - `generate`: build `crates/emoji-data/src/generated.rs` from
//!   emoji-test.txt + gemoji + emojilib (Phase 2).
//! - `render-icons`: rasterize `images/*.png` from the system emoji font
//!   (Phase 3, macOS only — see render_icons.rs for why).
//!
//! Must be run from the workspace root (paths are relative to it).

mod generate;

#[cfg(target_os = "macos")]
mod render_icons;

#[cfg(not(target_os = "macos"))]
mod render_icons {
    pub fn run() -> anyhow::Result<()> {
        anyhow::bail!("render-icons uses CoreText and only runs on macOS")
    }
}

fn main() -> anyhow::Result<()> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("generate") => generate::run(),
        Some("render-icons") => render_icons::run(),
        _ => {
            eprintln!("usage: cargo run -p xtask -- <generate|render-icons>");
            std::process::exit(2);
        }
    }
}
