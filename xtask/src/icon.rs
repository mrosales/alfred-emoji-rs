//! `xtask icon`: rasterize the workflow's own `icon.png` (shown in
//! Alfred's workflow list) from the system emoji font — same renderer as
//! `render-icons`, just a single glyph at a higher resolution suited to
//! a standalone icon rather than a small in-list image. Alfred picks up
//! `icon.png` by filename at the bundle root; no info.plist entry needed.

use std::path::Path;

use anyhow::{bail, Result};

use crate::emoji_render::{render, RenderOutcome};

const OUT_PATH: &str = "icon.png";
const CANVAS_PX: i32 = 512;
const POINT_SIZE: f64 = 420.0;
const ROCKET: &str = "\u{1F680}";

pub fn run() -> Result<()> {
    match render(ROCKET, POINT_SIZE, CANVAS_PX, Path::new(OUT_PATH))? {
        RenderOutcome::Written => {
            eprintln!("wrote {OUT_PATH}");
            Ok(())
        }
        RenderOutcome::Unsupported => bail!("the installed font can't render {ROCKET:?}"),
    }
}
