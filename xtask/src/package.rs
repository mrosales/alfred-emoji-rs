//! `xtask package`: build the release binary and zip it with `info.plist`
//! and `images/` into a `.alfredworkflow` bundle (PLAN.md Phase 6).
//! `--sign` additionally code-signs and notarizes the staged binary
//! before zipping (see `notarize.rs`) — the shipped-release path, not the
//! default, since it costs a real round trip to Apple's notary service.
//!
//! Deliberately does *not* run `generate`/`render-icons` itself — those
//! are their own deliberate steps (see generate.rs's header), and running
//! a full icon re-render as a side effect of packaging would be
//! surprising and slow. Run them first; this just errors clearly if
//! `images/` is missing.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::build::{release_binary, BINARY_NAME};
use crate::notarize::sign_and_notarize;

const STAGING_DIR: &str = "target/package";
const DIST_DIR: &str = "dist";
const WORKFLOW_NAME: &str = "alfred-emoji-rs.alfredworkflow";

pub fn run(sign: bool) -> Result<()> {
    if !Path::new("images").is_dir() {
        bail!(
            "images/ not found — run `cargo run -p xtask -- render-icons` first \
             (see PLAN.md Phase 3)"
        );
    }

    let binary = release_binary()?;

    let staging = Path::new(STAGING_DIR);
    if staging.exists() {
        std::fs::remove_dir_all(staging).context("clearing stale staging directory")?;
    }
    std::fs::create_dir_all(staging).context("creating staging directory")?;

    std::fs::copy("info.plist", staging.join("info.plist")).context("copying info.plist")?;
    copy_dir_recursive(Path::new("images"), &staging.join("images")).context("copying images/")?;
    let staged_binary = staging.join(BINARY_NAME);
    std::fs::copy(&binary, &staged_binary).context("copying release binary")?;

    // Must stay here, on the staged copy, as the last thing that touches
    // it before zipping — see notarize.rs's header for why.
    if sign {
        sign_and_notarize(&staged_binary)?;
    }

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
