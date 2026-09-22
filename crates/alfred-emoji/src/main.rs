//! Alfred script filter entry point. Search/ranking (Phase 4) and Alfred
//! JSON output (Phase 5) land in later phases — this is scaffolding only.

fn main() {
    let query = std::env::args().nth(1).unwrap_or_default();
    let _ = emoji_data::EMOJIS;
    eprintln!("alfred-emoji: not yet implemented (query = {query:?})");
    std::process::exit(1);
}
