//! Emoji dataset and search index.
//!
//! The actual dataset (`generated.rs`) is produced by `xtask generate`
//! (Phase 2) and is not checked in yet — this crate currently only defines
//! the shapes that step will populate.

mod model;

pub use model::{Emoji, ImageData, SkinTone};

/// Placeholder until `xtask generate` produces `generated.rs`.
pub static EMOJIS: &[Emoji] = &[];
