//! Alfred script filter entry point: `alfred-emoji <query>`, reading the
//! `skin_tone` workflow variable from the environment, printing Alfred
//! script filter JSON (alfred.rs) to stdout. Mirrors the UX contract of
//! `alfred-emoji-go`'s `main.go` — paste by default, ⌘ copies the
//! character, ⌥ copies the `:shortcode:` — see PLAN.md Phase 5.

mod alfred;
mod update;

use std::path::Path;

use alfred::{Icon, Item, Mod, Mods};
use emoji_data::{search, Emoji, ImageData, SkinTone};
use update::AvailableUpdate;

const IMAGE_DIR: &str = "images";
/// Alfred's own list UI is only comfortably scrollable so far; a broad
/// query can otherwise fuzzy-match most of the ~1,900-emoji dataset at
/// very low relevance. Not part of SPEC.md — a pragmatic cap so the JSON
/// payload and Alfred's rendering stay snappy.
const MAX_RESULTS: usize = 50;

fn parse_skin_tone(value: &str) -> Result<Option<SkinTone>, ()> {
    match value {
        "" => Ok(None),
        "light" => Ok(Some(SkinTone::Light)),
        "medium_light" => Ok(Some(SkinTone::MediumLight)),
        "medium" => Ok(Some(SkinTone::Medium)),
        "medium_dark" => Ok(Some(SkinTone::MediumDark)),
        "dark" => Ok(Some(SkinTone::Dark)),
        _ => Err(()),
    }
}

fn icon_for(image: &ImageData) -> Option<Icon> {
    let path = format!("{IMAGE_DIR}/{}.png", image.unified);
    // Phase 3 skips rendering codepoints the local font can't map yet
    // (see render_icons.rs) — fall back to Alfred's default icon for
    // those rather than pointing at a PNG that doesn't exist.
    Path::new(&path).exists().then_some(Icon { path })
}

fn build_item(emoji: &Emoji, skin_tone: Option<SkinTone>) -> Item {
    let image = emoji.image_for(skin_tone);
    let character = image.character.to_string();
    let name = emoji.name;

    Item {
        uid: Some(name.to_string()),
        title: name.to_string(),
        subtitle: format!("Paste symbol \"{character}\" in frontmost app"),
        arg: Some(character.clone()),
        valid: true,
        icon: icon_for(&image),
        variables: Some([("action", "paste")].into()),
        mods: Some(Mods {
            cmd: Mod {
                subtitle: format!("Copy symbol \"{character}\" to the clipboard"),
                arg: character.clone(),
                variables: [("action", "copy")].into(),
            },
            alt: Mod {
                subtitle: format!("Copy code \":{name}:\" to the clipboard"),
                arg: character,
                variables: [("action", "copy")].into(),
            },
        }),
    }
}

/// `--update-check <repository>`: the detached background process
/// `update::spawn_background_check` invokes on itself. Returns `true` when
/// it handled the invocation (so `main` should stop rather than search).
fn handle_update_check_subcommand() -> bool {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some(update::CHECK_ARG) {
        return false;
    }
    update::run_background_check(&args.next().unwrap_or_default());
    true
}

fn build_update_item(update: &AvailableUpdate) -> Item {
    Item {
        uid: None,
        title: format!("Update available: v{}", update.version),
        subtitle: "↩ to download and install the new version".to_string(),
        arg: Some(update.download_url.clone()),
        valid: true,
        icon: None,
        variables: Some([("action", "install_update")].into()),
        mods: None,
    }
}

fn main() {
    if handle_update_check_subcommand() {
        return;
    }

    let query = std::env::args().nth(1).unwrap_or_default();
    let skin_tone_raw = std::env::var("skin_tone").unwrap_or_default();

    let skin_tone = match parse_skin_tone(&skin_tone_raw) {
        Ok(tone) => tone,
        Err(()) => {
            alfred::print_items(vec![Item::message(
                "Invalid skin_tone",
                format!("\"{skin_tone_raw}\" — check the workflow's skin_tone variable"),
            )]);
            return;
        }
    };

    let mut items = Vec::new();
    // Only on an empty query (i.e. right after typing the keyword) so an
    // update notice doesn't compete with every search's results.
    if query.is_empty() {
        if let Some(available) =
            update::check(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_REPOSITORY"))
        {
            items.push(build_update_item(&available));
        }
    }

    let hits = search(&query);
    if hits.is_empty() {
        items.push(Item::message("No matching items", "Try a different query?"));
        alfred::print_items(items);
        return;
    }

    items.extend(
        hits.into_iter()
            .take(MAX_RESULTS)
            .map(|hit| build_item(hit.emoji, skin_tone)),
    );
    alfred::print_items(items);
}

#[cfg(test)]
mod tests {
    use super::*;
    use emoji_data::SkinTone::*;

    #[test]
    fn parse_skin_tone_accepts_the_five_documented_values() {
        assert_eq!(parse_skin_tone(""), Ok(None));
        assert_eq!(parse_skin_tone("light"), Ok(Some(Light)));
        assert_eq!(parse_skin_tone("medium_light"), Ok(Some(MediumLight)));
        assert_eq!(parse_skin_tone("medium"), Ok(Some(Medium)));
        assert_eq!(parse_skin_tone("medium_dark"), Ok(Some(MediumDark)));
        assert_eq!(parse_skin_tone("dark"), Ok(Some(Dark)));
    }

    #[test]
    fn parse_skin_tone_rejects_anything_else() {
        assert_eq!(parse_skin_tone("purple"), Err(()));
        assert_eq!(parse_skin_tone("Light"), Err(())); // case-sensitive, matches the Go version
    }

    fn sample_emoji() -> Emoji {
        Emoji {
            name: "rocket",
            category: "Travel & Places",
            shortcodes: &["rocket"],
            keywords: &["space"],
            image: ImageData {
                unified: "1f680",
                character: "🚀",
            },
            skin_variations: &[],
        }
    }

    #[test]
    fn build_item_wires_up_default_paste_action() {
        let item = build_item(&sample_emoji(), None);
        assert_eq!(item.uid.as_deref(), Some("rocket"));
        assert_eq!(item.title, "rocket");
        assert_eq!(item.arg.as_deref(), Some("🚀"));
        assert!(item.valid);
        assert_eq!(item.subtitle, "Paste symbol \"🚀\" in frontmost app");
        assert_eq!(item.variables.unwrap()["action"], "paste");
    }

    #[test]
    fn build_item_wires_up_copy_and_copy_shortcode_mods() {
        let item = build_item(&sample_emoji(), None);
        let mods = item.mods.expect("expected cmd/alt mods");
        assert_eq!(mods.cmd.arg, "🚀");
        assert_eq!(mods.cmd.subtitle, "Copy symbol \"🚀\" to the clipboard");
        assert_eq!(mods.cmd.variables["action"], "copy");

        assert_eq!(mods.alt.arg, "🚀");
        assert_eq!(mods.alt.subtitle, "Copy code \":rocket:\" to the clipboard");
        assert_eq!(mods.alt.variables["action"], "copy");
    }
}
