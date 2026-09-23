//! FFI binding to `native/render_emoji.c`'s CoreText rasterizer, shared by
//! `render-icons` (per-emoji dataset icons) and `icon` (the workflow's
//! own icon.png).

use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int};
use std::path::Path;

use anyhow::{Context, Result};

extern "C" {
    fn render_emoji_png(
        utf8_text: *const c_char,
        point_size: c_double,
        canvas_px: c_int,
        out_path: *const c_char,
    ) -> c_int;
}

pub enum RenderOutcome {
    Written,
    Unsupported,
}

pub fn render(
    character: &str,
    point_size: c_double,
    canvas_px: c_int,
    out_path: &Path,
) -> Result<RenderOutcome> {
    let text = CString::new(character).context("emoji text contained a NUL byte")?;
    let path = CString::new(out_path.to_string_lossy().as_bytes())
        .context("output path contained a NUL byte")?;
    // SAFETY: `render_emoji_png` takes two borrowed, NUL-terminated C
    // strings it only reads for the duration of the call, and two plain
    // numeric args. Both CStrings outlive the call.
    let rc = unsafe { render_emoji_png(text.as_ptr(), point_size, canvas_px, path.as_ptr()) };
    match rc {
        0 => Ok(RenderOutcome::Written),
        1 => Ok(RenderOutcome::Unsupported),
        _ => anyhow::bail!("render_emoji_png failed for {character:?} -> {out_path:?}"),
    }
}
