//! Emoji dataset and search index.
//!
//! `generated.rs` is produced by `cargo run -p xtask -- generate` (Phase 2)
//! from Unicode's emoji-test.txt + gemoji + emojilib; see that file's
//! header for dataset provenance. It's committed so the crate builds
//! without a network fetch, and regenerated on demand.

mod generated;
mod model;
mod search;

pub use generated::{EMOJIS, SHORTCODE_INDEX};
pub use model::{Emoji, ImageData, SkinTone};
pub use search::{search, SearchHit};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_is_populated() {
        assert!(EMOJIS.len() > 1000, "expected a full emoji dataset");
    }

    #[test]
    fn shortcode_index_resolves_to_matching_emoji() {
        let idx = SHORTCODE_INDEX["rocket"];
        assert_eq!(EMOJIS[idx as usize].image.character, "\u{1f680}");
    }

    #[test]
    fn every_emoji_has_at_least_one_shortcode() {
        assert!(EMOJIS.iter().all(|e| !e.shortcodes.is_empty()));
    }

    #[test]
    fn emoji_18_fallback_entries_are_present_and_searchable() {
        // These shipped in Emoji 18.0 (2026-09-16) and are new enough that
        // gemoji/emojilib haven't curated aliases for them yet — PLAN.md §0
        // says they should still resolve via the slugified-name fallback.
        let idx = SHORTCODE_INDEX["pickle"];
        assert_eq!(EMOJIS[idx as usize].category, "Food & Drink");

        let idx = SHORTCODE_INDEX["cracking_face"];
        assert_eq!(EMOJIS[idx as usize].category, "Smileys & Emotion");
    }

    #[test]
    fn skin_tone_variation_resolves_correct_image() {
        let idx = SHORTCODE_INDEX["wave"];
        let emoji = &EMOJIS[idx as usize];
        let light = emoji.image_for(Some(SkinTone::Light));
        assert_ne!(light.character, emoji.image.character);
        assert_eq!(emoji.image_for(None).character, emoji.image.character);
    }
}
