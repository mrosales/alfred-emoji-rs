//! `xtask package`: build the release binary and zip it with `info.plist`
//! and `images/` into a `.alfredworkflow` bundle (PLAN.md Phase 6).
//!
//! Deliberately does *not* run `generate`/`render-icons` itself — those
//! are their own deliberate steps (see generate.rs's header), and running
//! a full icon re-render as a side effect of packaging would be
//! surprising and slow. Run them first; this just errors clearly if
//! `images/` is missing.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

const STAGING_DIR: &str = "target/package";
const DIST_DIR: &str = "dist";
const WORKFLOW_NAME: &str = "alfred-emoji-rs.alfredworkflow";
const BINARY_NAME: &str = "alfred-emoji";

pub fn run() -> Result<()> {
    if !Path::new("images").is_dir() {
        bail!(
            "images/ not found — run `cargo run -p xtask -- render-icons` first \
             (see PLAN.md Phase 3)"
        );
    }

    eprintln!("building {BINARY_NAME} (release)");
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", BINARY_NAME])
        .status()
        .context("running cargo build --release")?;
    if !status.success() {
        bail!("cargo build --release failed");
    }

    let staging = Path::new(STAGING_DIR);
    if staging.exists() {
        std::fs::remove_dir_all(staging).context("clearing stale staging directory")?;
    }
    std::fs::create_dir_all(staging).context("creating staging directory")?;

    std::fs::copy("info.plist", staging.join("info.plist")).context("copying info.plist")?;
    copy_dir_recursive(Path::new("images"), &staging.join("images")).context("copying images/")?;
    std::fs::copy(
        format!("target/release/{BINARY_NAME}"),
        staging.join(BINARY_NAME),
    )
    .context("copying release binary")?;

    std::fs::create_dir_all(DIST_DIR).context("creating dist/")?;
    let workflow_path = Path::new(DIST_DIR)
        .canonicalize()
        .context("resolving dist/ path")?
        .join(WORKFLOW_NAME);
    if workflow_path.exists() {
        std::fs::remove_file(&workflow_path).context("removing stale .alfredworkflow")?;
    }

    // Zip the staging directory's *contents* (info.plist at the archive
    // root, not nested under target/package/) — Alfred expects info.plist
    // at the top level of the bundle.
    let status = Command::new("zip")
        .args(["-r", "-X", "-q"])
        .arg(&workflow_path)
        .arg(".")
        .current_dir(staging)
        .status()
        .context("running zip")?;
    if !status.success() {
        bail!("zip failed");
    }

    eprintln!("wrote {}", workflow_path.display());
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
