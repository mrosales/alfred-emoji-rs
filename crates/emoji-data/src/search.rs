//! Fuzzy + alias search, per SPEC.md §4: an exact-shortcode fast path,
//! then two weighted `nucleo` fuzzy passes (shortcodes outrank keywords),
//! merged per emoji by max score (not sum) so one tight match beats many
//! loose ones.

use std::collections::HashMap;

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::{Emoji, EMOJIS, SHORTCODE_INDEX};

/// Shortcode matches are weighted above keyword matches of similar fuzzy
/// quality — shortcodes are curated and precise, keywords are long-tail
/// and noisier (SPEC.md §4).
const SHORTCODE_WEIGHT: u32 = 3;
const KEYWORD_WEIGHT: u32 = 1;

pub struct SearchHit {
    pub emoji: &'static Emoji,
    pub score: u32,
}

/// The max (never sum) weighted score across `terms`, or `None` if none of
/// them match `pattern` at all. Taking the max is what stops an emoji with
/// twelve loosely-related keywords from outranking one with a single tight
/// match (SPEC.md §4).
fn best_score(
    pattern: &Pattern,
    matcher: &mut Matcher,
    buf: &mut Vec<char>,
    terms: &[&str],
    weight: u32,
) -> Option<u32> {
    let mut best: Option<u32> = None;
    for term in terms {
        let haystack = Utf32Str::new(term, buf);
        if let Some(score) = pattern.score(haystack, matcher) {
            let weighted = score * weight;
            best = Some(best.map_or(weighted, |b| b.max(weighted)));
        }
    }
    best
}

/// Search the dataset for `query`. Empty (after stripping any `:`
/// wrapping) returns no results. An exact shortcode match short-circuits
/// and is returned alone, ranked above anything fuzzy matching could find.
pub fn search(query: &str) -> Vec<SearchHit> {
    let query = query.trim().trim_matches(':');
    if query.is_empty() {
        return Vec::new();
    }

    let lower = query.to_lowercase();
    if let Some(&idx) = SHORTCODE_INDEX.get(lower.as_str()) {
        return vec![SearchHit {
            emoji: &EMOJIS[idx as usize],
            score: u32::MAX,
        }];
    }

    let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();

    let mut best: HashMap<usize, u32> = HashMap::new();
    for (i, emoji) in EMOJIS.iter().enumerate() {
        let shortcode_score = best_score(
            &pattern,
            &mut matcher,
            &mut buf,
            emoji.shortcodes,
            SHORTCODE_WEIGHT,
        );
        let keyword_score = best_score(
            &pattern,
            &mut matcher,
            &mut buf,
            emoji.keywords,
            KEYWORD_WEIGHT,
        );
        let merged = match (shortcode_score, keyword_score) {
            (None, None) => None,
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (Some(a), Some(b)) => Some(a.max(b)),
        };
        if let Some(score) = merged {
            best.insert(i, score);
        }
    }

    let mut hits: Vec<SearchHit> = best
        .into_iter()
        .map(|(i, score)| SearchHit {
            emoji: &EMOJIS[i],
            score,
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.emoji.name.cmp(b.emoji.name))
    });
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_returns_no_hits() {
        assert!(search("").is_empty());
        assert!(search("   ").is_empty());
        assert!(search("::").is_empty());
    }

    #[test]
    fn exact_shortcode_match_short_circuits() {
        let hits = search("rocket");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].emoji.image.character, "\u{1f680}");
        assert_eq!(hits[0].score, u32::MAX);
    }

    #[test]
    fn exact_match_is_case_insensitive_and_colon_wrapped() {
        let hits = search(":ROCKET:");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].emoji.image.character, "\u{1f680}");
    }

    #[test]
    fn fuzzy_prefix_query_ranks_prefix_matches_at_the_top() {
        // "roc" (not "rock" — that's itself an exact shortcode for 🪨 and
        // would hit the short-circuit path instead of the fuzzy pass).
        // "rock" and "rocket" both prefix-match and tie in nucleo's
        // scoring, so assert the pair leads rather than picking a winner
        // between two correctly-tied results.
        let hits = search("roc");
        let top_names: Vec<&str> = hits.iter().take(2).map(|h| h.emoji.name).collect();
        assert!(top_names.contains(&"rock"), "{top_names:?}");
        assert!(top_names.contains(&"rocket"), "{top_names:?}");
    }

    #[test]
    fn keyword_only_match_is_found() {
        // "farewell" is a gemoji tag on waving hand, not a shortcode —
        // this only resolves if the keyword pass runs at all.
        let hits = search("farewell");
        assert!(!hits.is_empty());
        assert!(!hits[0].emoji.shortcodes.contains(&"farewell"));
        assert!(hits[0].emoji.keywords.contains(&"farewell"));
    }

    #[test]
    fn no_match_returns_empty() {
        assert!(search("qqqzzzxxx1234").is_empty());
    }

    #[test]
    fn best_score_takes_max_not_sum() {
        let pattern = Pattern::parse("cat", CaseMatching::Ignore, Normalization::Smart);
        let mut matcher = Matcher::new(Config::DEFAULT);
        let mut buf = Vec::new();

        let single = best_score(&pattern, &mut matcher, &mut buf, &["cat"], 1).unwrap();
        let many = best_score(
            &pattern,
            &mut matcher,
            &mut buf,
            &["cat", "cats", "concatenate", "vacate"],
            1,
        )
        .unwrap();

        // "cat" is already the best any of these terms can score against
        // itself, so max-of-terms must equal it exactly. If this were
        // summed across every matching term instead, `many` would exceed
        // `single`.
        assert_eq!(single, many, "expected max-of-terms, not sum-of-terms");
    }

    #[test]
    fn emoji_18_fallback_entry_is_fuzzy_searchable() {
        let hits = search("pickl");
        assert!(!hits.is_empty());
        assert!(hits[0].emoji.shortcodes.contains(&"pickle"));
    }
}
