//! Task runner: `cargo run -p xtask -- <command>`.
//!
//! - `generate`: build `crates/emoji-data/src/generated.rs` from
//!   emoji-test.txt + gemoji + emojilib (Phase 2).
//! - `render-icons`: rasterize `images/*.png` from the system emoji font
//!   (Phase 3, not yet implemented, macOS only).
//!
//! Must be run from the workspace root (paths are relative to it).

mod generate;

fn main() -> anyhow::Result<()> {
    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("generate") => generate::run(),
        Some("render-icons") => {
            eprintln!("xtask render-icons: not yet implemented (Phase 3)");
            std::process::exit(1);
        }
        _ => {
            eprintln!("usage: cargo run -p xtask -- <generate|render-icons>");
            std::process::exit(2);
        }
    }
}
