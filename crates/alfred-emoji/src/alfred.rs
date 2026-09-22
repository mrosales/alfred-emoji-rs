//! Minimal Alfred 5 script filter JSON output.
//!
//! PLAN.md Phase 5 chose to hand-roll this rather than port an AwGo-
//! equivalent crate: the actual surface `alfred-emoji-go`'s `main.go`
//! used was small (items, subtitle, icon, arg, variables, uid, cmd/alt
//! mods) — see <https://www.alfredapp.com/help/workflows/inputs/script-filter/json/>
//! for the full schema this is a subset of.

use std::collections::HashMap;
use std::io::Write;

use serde::Serialize;

#[derive(Serialize)]
pub struct Output {
    pub items: Vec<Item>,
}

#[derive(Serialize)]
pub struct Item {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
    pub title: String,
    pub subtitle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arg: Option<String>,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<&'static str, &'static str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mods: Option<Mods>,
}

#[derive(Serialize)]
pub struct Icon {
    pub path: String,
}

#[derive(Serialize)]
pub struct Mods {
    pub cmd: Mod,
    pub alt: Mod,
}

#[derive(Serialize)]
pub struct Mod {
    pub subtitle: String,
    pub arg: String,
    pub variables: HashMap<&'static str, &'static str>,
}

impl Item {
    /// A non-actionable informational row (no matches, or a bad env var) —
    /// what `wf.WarnEmpty`/`wf.FatalError` rendered in the Go version.
    pub fn message(title: impl Into<String>, subtitle: impl Into<String>) -> Self {
        Item {
            uid: None,
            title: title.into(),
            subtitle: subtitle.into(),
            arg: None,
            valid: false,
            icon: None,
            variables: None,
            mods: None,
        }
    }
}

/// Serializes `items` as Alfred script filter JSON to stdout.
pub fn print_items(items: Vec<Item>) {
    let output = Output { items };
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    // A script filter's JSON is consumed by Alfred, not a human; failure to
    // write it means Alfred already lost the pipe, and there's nothing
    // sensible left to fall back to (i.e. not worth an panic-free error
    // path with no recovery action a caller could take).
    serde_json::to_writer(&mut lock, &output).expect("failed to serialize Alfred JSON output");
    let _ = lock.write_all(b"\n");
}
