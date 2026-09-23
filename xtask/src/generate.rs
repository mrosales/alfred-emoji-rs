//! `xtask generate`: merge Unicode emoji-test.txt (source of truth for the
//! codepoint list, per PLAN.md §0 — iamcal/gemoji/emojilib lag a fresh
//! Unicode release) with gemoji (shortcodes/tags) and emojilib (extra
//! keywords), and emit `crates/emoji-data/src/generated.rs`.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

// Pinned to Emoji 17.0 rather than `/emoji/latest/` (currently Emoji 18.0):
// macOS does not yet ship glyphs for Emoji 18 additions, so 18.0 codepoints
// would render as fallback boxes. Unicode hasn't published a
// `/Public/emoji/17.0/` archive yet, but the same file is mirrored under the
// matching UCD release tree.
const EMOJI_TEST_URL: &str = "https://unicode.org/Public/17.0.0/emoji/emoji-test.txt";
const GEMOJI_URL: &str = "https://raw.githubusercontent.com/github/gemoji/master/db/emoji.json";
const EMOJILIB_URL: &str =
    "https://raw.githubusercontent.com/muan/emojilib/main/dist/emoji-en-US.json";

const MAX_KEYWORDS: usize = 12;
const GENERATED_PATH: &str = "crates/emoji-data/src/generated.rs";

const SKIN_TONES: [(u32, &str); 5] = [
    (0x1F3FB, "Light"),
    (0x1F3FC, "MediumLight"),
    (0x1F3FD, "Medium"),
    (0x1F3FE, "MediumDark"),
    (0x1F3FF, "Dark"),
];

#[derive(Deserialize)]
struct GemojiRaw {
    emoji: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

struct RawEntry {
    codepoints: Vec<u32>,
    name: String,
    group: String,
}

struct BuiltEmoji {
    name: String,
    category: String,
    shortcodes: Vec<String>,
    keywords: Vec<String>,
    unified: String,
    character: String,
    skin_variations: Vec<(&'static str, String, String)>, // (tone variant, unified, character)
}

pub fn run() -> Result<()> {
    eprintln!("fetching {EMOJI_TEST_URL}");
    let emoji_test = fetch_text(EMOJI_TEST_URL)?;
    let unicode_version = emoji_test
        .lines()
        .find_map(|l| l.strip_prefix("# Version: "))
        .unwrap_or("unknown")
        .trim()
        .to_string();

    eprintln!("fetching {GEMOJI_URL}");
    let gemoji: Vec<GemojiRaw> =
        serde_json::from_str(&fetch_text(GEMOJI_URL)?).context("parsing gemoji emoji.json")?;
    let gemoji_by_char: HashMap<String, &GemojiRaw> =
        gemoji.iter().map(|g| (g.emoji.clone(), g)).collect();

    eprintln!("fetching {EMOJILIB_URL}");
    let emojilib: HashMap<String, Vec<String>> =
        serde_json::from_str(&fetch_text(EMOJILIB_URL)?).context("parsing emojilib json")?;

    let entries = parse_emoji_test(&emoji_test);
    let built = merge(entries, &gemoji_by_char, &emojilib);

    eprintln!(
        "merged {} emoji ({} with gemoji coverage, {} with emojilib coverage, {} fallback-only)",
        built.len(),
        built
            .iter()
            .filter(|e| gemoji_by_char.contains_key(&e.character))
            .count(),
        built
            .iter()
            .filter(|e| emojilib.contains_key(&e.character))
            .count(),
        built
            .iter()
            .filter(|e| !gemoji_by_char.contains_key(&e.character)
                && !emojilib.contains_key(&e.character))
            .count(),
    );

    let source = render(&built, &unicode_version);
    let out_path = Path::new(GENERATED_PATH);
    std::fs::write(out_path, source).with_context(|| format!("writing {}", out_path.display()))?;
    eprintln!("wrote {}", out_path.display());
    Ok(())
}

fn fetch_text(url: &str) -> Result<String> {
    ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?
        .into_string()
        .with_context(|| format!("reading body of {url}"))
}

fn skin_tone_for(cp: u32) -> Option<&'static str> {
    SKIN_TONES
        .iter()
        .find(|(code, _)| *code == cp)
        .map(|(_, name)| *name)
}

fn parse_emoji_test(text: &str) -> Vec<RawEntry> {
    let mut current_group = String::from("Uncategorized");
    let mut entries = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            if let Some(group) = rest.trim().strip_prefix("group:") {
                current_group = group.trim().to_string();
            }
            continue;
        }
        let Some((cp_part, rest)) = line.split_once(';') else {
            continue;
        };
        let Some((status_part, comment)) = rest.split_once('#') else {
            continue;
        };
        if status_part.trim() != "fully-qualified" {
            continue;
        }
        let Ok(codepoints) = cp_part
            .split_whitespace()
            .map(|h| u32::from_str_radix(h, 16))
            .collect::<Result<Vec<_>, _>>()
        else {
            continue;
        };
        // comment is "<rendered glyph> E<version> <name...>" — skip the
        // glyph and version tokens, keep the rest as the name.
        let name: String = comment
            .split_whitespace()
            .skip(2)
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            continue;
        }
        entries.push(RawEntry {
            codepoints,
            name,
            group: current_group.clone(),
        });
    }
    entries
}

fn codepoints_to_string(codepoints: &[u32]) -> String {
    codepoints
        .iter()
        .filter_map(|&cp| char::from_u32(cp))
        .collect()
}

fn unified(codepoints: &[u32]) -> String {
    codepoints
        .iter()
        .map(|cp| format!("{cp:x}"))
        .collect::<Vec<_>>()
        .join("-")
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = true; // suppress leading underscore
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn merge(
    entries: Vec<RawEntry>,
    gemoji_by_char: &HashMap<String, &GemojiRaw>,
    emojilib: &HashMap<String, Vec<String>>,
) -> Vec<BuiltEmoji> {
    // Split into base entries and single-skin-tone variants; drop entries
    // with 2+ skin-tone modifiers (couple/family dual-tone combinatorics —
    // out of scope, see PLAN.md, matches the Go version's single-Modifier
    // skin tone model).
    let mut base_entries: Vec<RawEntry> = Vec::new();
    let mut variants: HashMap<String, Vec<(&'static str, String, String)>> = HashMap::new();

    for entry in entries {
        let modifier_positions: Vec<(usize, &'static str)> = entry
            .codepoints
            .iter()
            .enumerate()
            .filter_map(|(i, &cp)| skin_tone_for(cp).map(|t| (i, t)))
            .collect();

        match modifier_positions.len() {
            0 => base_entries.push(entry),
            1 => {
                let (idx, tone) = modifier_positions[0];
                let base_codepoints: Vec<u32> = entry
                    .codepoints
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| *i != idx)
                    .map(|(_, &cp)| cp)
                    .collect();
                let key = unified(&base_codepoints);
                variants.entry(key).or_default().push((
                    tone,
                    unified(&entry.codepoints),
                    codepoints_to_string(&entry.codepoints),
                ));
            }
            _ => continue,
        }
    }

    let mut seen = HashSet::new();
    let mut built = Vec::with_capacity(base_entries.len());

    for entry in base_entries {
        let key = unified(&entry.codepoints);
        if !seen.insert(key.clone()) {
            continue;
        }
        let character = codepoints_to_string(&entry.codepoints);
        let gemoji = gemoji_by_char.get(&character).copied();
        let lib_keywords = emojilib.get(&character);

        let fallback_slug = slugify(&entry.name);

        let mut shortcodes: Vec<String> = Vec::new();
        if let Some(g) = gemoji {
            shortcodes.extend(g.aliases.iter().cloned());
        }
        if let Some(kw) = lib_keywords.and_then(|k| k.first()) {
            shortcodes.push(kw.clone());
        }
        if shortcodes.is_empty() {
            shortcodes.push(fallback_slug.clone());
        }
        dedup_case_insensitive(&mut shortcodes);

        let taken: HashSet<String> = shortcodes.iter().map(|s| s.to_lowercase()).collect();
        let mut keywords: Vec<String> = Vec::new();
        if let Some(g) = gemoji {
            keywords.extend(g.tags.iter().cloned());
        }
        if let Some(kw) = lib_keywords {
            keywords.extend(kw.iter().skip(1).cloned());
        }
        if keywords.is_empty() && gemoji.is_none() && lib_keywords.is_none() {
            keywords.push(fallback_slug);
        }
        keywords.retain(|k| !taken.contains(&k.to_lowercase()));
        dedup_case_insensitive(&mut keywords);
        keywords.truncate(MAX_KEYWORDS);

        let name = shortcodes
            .first()
            .cloned()
            .unwrap_or_else(|| slugify(&entry.name));

        built.push(BuiltEmoji {
            name,
            category: entry.group,
            shortcodes,
            keywords,
            unified: key.clone(),
            character,
            skin_variations: variants.remove(&key).unwrap_or_default(),
        });
    }

    built
}

fn dedup_case_insensitive(items: &mut Vec<String>) {
    let mut seen = HashSet::new();
    items.retain(|item| seen.insert(item.to_lowercase()));
}

fn render(built: &[BuiltEmoji], unicode_version: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "// GENERATED FILE — do not edit by hand.");
    let _ = writeln!(out, "// Produced by `cargo run -p xtask -- generate`.");
    let _ = writeln!(out, "//");
    let _ = writeln!(out, "// Sources:");
    let _ = writeln!(
        out,
        "//   Unicode emoji-test.txt, Version: {unicode_version} ({EMOJI_TEST_URL})"
    );
    let _ = writeln!(out, "//   gemoji db/emoji.json ({GEMOJI_URL})");
    let _ = writeln!(out, "//   emojilib emoji-en-US.json ({EMOJILIB_URL})");
    let _ = writeln!(out);
    let _ = writeln!(out, "use crate::{{Emoji, ImageData, SkinTone}};");
    let _ = writeln!(out);
    let _ = writeln!(out, "pub static EMOJIS: &[Emoji] = &[");
    for e in built {
        let _ = writeln!(out, "    Emoji {{");
        let _ = writeln!(out, "        name: {:?},", e.name);
        let _ = writeln!(out, "        category: {:?},", e.category);
        let _ = writeln!(out, "        shortcodes: &{:?},", e.shortcodes);
        let _ = writeln!(out, "        keywords: &{:?},", e.keywords);
        let _ = writeln!(
            out,
            "        image: ImageData {{ unified: {:?}, character: {:?} }},",
            e.unified, e.character
        );
        if e.skin_variations.is_empty() {
            let _ = writeln!(out, "        skin_variations: &[],");
        } else {
            let _ = writeln!(out, "        skin_variations: &[");
            for (tone, unified, character) in &e.skin_variations {
                let _ = writeln!(
                    out,
                    "            (SkinTone::{tone}, ImageData {{ unified: {unified:?}, character: {character:?} }}),"
                );
            }
            let _ = writeln!(out, "        ],");
        }
        let _ = writeln!(out, "    }},");
    }
    let _ = writeln!(out, "];");
    let _ = writeln!(out);

    let mut used_shortcodes: HashSet<String> = HashSet::new();
    let mut index_entries: Vec<(String, usize)> = Vec::new();
    let mut collisions = 0u32;
    for (i, e) in built.iter().enumerate() {
        for code in &e.shortcodes {
            let key = code.to_lowercase();
            if used_shortcodes.insert(key.clone()) {
                index_entries.push((key, i));
            } else {
                collisions += 1;
            }
        }
    }
    if collisions > 0 {
        eprintln!(
            "warning: {collisions} shortcode collisions found; first emoji to claim a \
             shortcode wins the exact-match index (all emoji keep their full shortcode \
             list for fuzzy search regardless)"
        );
    }

    let _ = writeln!(
        out,
        "pub static SHORTCODE_INDEX: phf::Map<&'static str, u16> = phf::phf_map! {{"
    );
    for (code, idx) in &index_entries {
        let _ = writeln!(out, "    {code:?} => {idx}u16,");
    }
    let _ = writeln!(out, "}};");

    out
}
