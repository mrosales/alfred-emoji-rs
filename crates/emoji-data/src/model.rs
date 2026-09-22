//! Data shapes shared between the dataset generator (`xtask`) and the
//! search index. See SPEC.md §3.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinTone {
    Light,
    MediumLight,
    Medium,
    MediumDark,
    Dark,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageData {
    /// Hyphen-separated hex codepoints, including any ZWJ.
    pub unified: &'static str,
    /// The actual emoji character (one or more codepoints).
    pub character: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct Emoji {
    /// Canonical short name (from Unicode's `emoji-test.txt` description,
    /// or iamcal's short name where available).
    pub name: &'static str,
    pub category: &'static str,
    /// Shortcodes: gemoji aliases + iamcal short names, highest precision.
    pub shortcodes: &'static [&'static str],
    /// Broader keywords: gemoji tags + filtered emojilib terms.
    pub keywords: &'static [&'static str],
    pub image: ImageData,
    pub skin_variations: &'static [(SkinTone, ImageData)],
}

impl Emoji {
    pub fn image_for(&self, tone: Option<SkinTone>) -> ImageData {
        match tone {
            Some(tone) => self
                .skin_variations
                .iter()
                .find(|(t, _)| *t == tone)
                .map(|(_, image)| *image)
                .unwrap_or(self.image),
            None => self.image,
        }
    }
}
