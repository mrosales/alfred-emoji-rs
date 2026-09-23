//! Builds the release `alfred-emoji` binary; shared by `package`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const BINARY_NAME: &str = "alfred-emoji";

pub fn release_binary() -> Result<PathBuf> {
    eprintln!("building {BINARY_NAME} (release)");
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", BINARY_NAME])
        .status()
        .context("running cargo build --release")?;
    if !status.success() {
        bail!("cargo build --release failed");
    }
    Ok(Path::new("target/release").join(BINARY_NAME))
}
