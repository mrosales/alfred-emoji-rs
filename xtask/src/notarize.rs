//! Code-signs a binary with a Developer ID Application certificate and
//! submits it to Apple's notary service, so Gatekeeper accepts it when
//! quarantined.
//!
//! `com.apple.quarantine` is set by browser/Finder downloads, not by
//! `curl` — only matters for someone grabbing the release
//! .alfredworkflow from GitHub directly rather than through the
//! in-workflow auto-updater (`update.rs`).
//!
//! No staple step: `xcrun stapler staple` only supports .app/.pkg/.dmg,
//! not a bare Mach-O binary. Gatekeeper instead checks the notarization
//! ticket online on first run (needs network once) and caches the result.
//!
//! Must be the last thing that touches the binary: `cargo build` relinks
//! a codesigned artifact on its next run, even with unchanged sources,
//! overwriting this signature with its own ad-hoc one. Sign a copy
//! nothing will `cargo build` again afterward (see `package.rs`).
//!
//! One-time signing-machine setup: a "Developer ID Application"
//! certificate in the login keychain, and `notarytool` credentials
//! stored under a named profile (`xcrun notarytool store-credentials
//! <profile> --key <path.p8> --key-id <id> --issuer <id>`). Override the
//! defaults via `ALFRED_EMOJI_SIGNING_IDENTITY` /
//! `ALFRED_EMOJI_NOTARY_PROFILE`.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

const SIGNING_IDENTITY_ENV: &str = "ALFRED_EMOJI_SIGNING_IDENTITY";
const DEFAULT_SIGNING_IDENTITY: &str = "Developer ID Application: Micah Rosales (ZFT3A4WCFY)";
const NOTARY_PROFILE_ENV: &str = "ALFRED_EMOJI_NOTARY_PROFILE";
const DEFAULT_NOTARY_PROFILE: &str = "personal-development-notary";

pub fn sign_and_notarize(binary: &Path) -> Result<()> {
    let identity = std::env::var(SIGNING_IDENTITY_ENV)
        .unwrap_or_else(|_| DEFAULT_SIGNING_IDENTITY.to_string());
    let profile =
        std::env::var(NOTARY_PROFILE_ENV).unwrap_or_else(|_| DEFAULT_NOTARY_PROFILE.to_string());

    eprintln!("codesigning {} as {identity:?}", binary.display());
    let status = Command::new("codesign")
        .args(["--force", "--options", "runtime", "--timestamp", "--sign"])
        .arg(&identity)
        .arg(binary)
        .status()
        .context(
            "running codesign — is the Developer ID Application cert in your login keychain?",
        )?;
    if !status.success() {
        bail!("codesign failed");
    }

    // notarytool requires a .zip/.pkg/.dmg, not a bare binary; `ditto -c
    // -k` zips it without disturbing the signature.
    let submission_zip = binary.with_file_name("alfred-emoji-notarize-submission.zip");
    if submission_zip.exists() {
        std::fs::remove_file(&submission_zip).context("removing stale submission zip")?;
    }
    let status = Command::new("ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(binary)
        .arg(&submission_zip)
        .status()
        .context("running ditto")?;
    if !status.success() {
        bail!("ditto (zipping the binary for notarization submission) failed");
    }

    eprintln!("submitting to notarytool (profile {profile:?}) — this waits on Apple, usually a few minutes");
    let output = Command::new("xcrun")
        .args(["notarytool", "submit"])
        .arg(&submission_zip)
        .args(["--keychain-profile", &profile, "--wait"])
        .output()
        .context("running notarytool submit — is the credentials profile stored?")?;
    std::fs::remove_file(&submission_zip).ok();

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{stdout}");
    if !output.status.success() || !stdout.contains("status: Accepted") {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        bail!(
            "notarization did not succeed — run `xcrun notarytool log <id> --keychain-profile {profile}` \
             with the submission id printed above for details"
        );
    }

    eprintln!(
        "notarization accepted; {} is ready to ship",
        binary.display()
    );
    Ok(())
}
