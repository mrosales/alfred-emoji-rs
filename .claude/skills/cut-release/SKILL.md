---
name: cut-release
description: Tag and publish a new alfred-emoji-rs release — bump the version, build the .alfredworkflow, and publish it as a GitHub release asset. Use when asked to cut/ship/publish a release, tag a new version, or release the workflow.
---

# Cut a release

Publishes a new version of the workflow: bumps the version everywhere it's
recorded, builds `dist/alfred-emoji-rs.alfredworkflow`, tags it, and
attaches the bundle to a GitHub release. This is the *producer* side of
the auto-update feature in `crates/alfred-emoji/src/update.rs` — that
module polls `GET /repos/mrosales/alfred-emoji-rs/releases/latest` and
expects to find exactly one `.alfredworkflow` asset there, so both
contracts below matter, not just "make a release exist."

## Contract this skill must uphold

1. **Version stays in sync in three places**: `Cargo.toml`
   (`workspace.package.version`), `info.plist` (`version` key), and the
   git tag (`vX.Y.Z`, `v` prefix). `update.rs` compares
   `env!("CARGO_PKG_VERSION")` against the release's `tag_name` with the
   leading `v` stripped — if these drift, the auto-update check either
   nags users who are already current or never fires.
2. **Exactly one `.alfredworkflow` asset per release.** `update.rs`'s
   `fetch_latest_release` takes the first asset whose name ends in
   `.alfredworkflow`; a second one is silently ambiguous. Don't attach
   anything else with that extension.
3. **The release must be built locally**, not in CI. `xtask render-icons`
   links against CoreText and needs to run on the machine whose emoji font
   is being rasterized (see README.md and the `refresh-emoji-data`
   skill) — CI only runs `clippy`/`test` (see `.github/workflows/ci.yml`),
   it doesn't build workflow bundles.

## Procedure

### 1. Preflight

```bash
git status --porcelain   # must be empty — see step 2 if not
git branch --show-current   # must be main
git fetch origin main
git rev-list --left-right --count origin/main...HEAD   # both sides 0
```

If the tree is dirty, stop and ask the user whether to commit, stash, or
abort — don't guess. If `main` is behind `origin/main`, pull first.

### 2. Pick the new version

Default to a patch bump of the current `workspace.package.version` in
`Cargo.toml`, unless the user's invocation names an explicit version
(e.g. `/cut-release 0.2.0`) or a bump kind (`minor`, `major`). State the
version you're about to cut before proceeding — this is a visible,
hard-to-reverse action (a public GitHub release), so don't just assume
silently.

### 3. Bump the version in both files

- `Cargo.toml`: `workspace.package.version = "X.Y.Z"`.
- `info.plist`: the top-level `version` string key (not any `version`
  integer key inside an object's config — those are Alfred's own object
  schema versions and are unrelated).

Run `cargo check --workspace` afterward — a version bump alone can't break
the build, but it regenerates `Cargo.lock`'s version fields, which should
be committed too.

### 4. Build and verify

```bash
# images/ isn't committed — build it fresh if missing or the user asks
# for a full refresh (see the refresh-emoji-data skill for when that's
# warranted; don't run it speculatively here).
test -d images || cargo run -p xtask -- render-icons

cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p xtask -- package
```

All four must succeed before continuing. `package` prints
`wrote /…/dist/alfred-emoji-rs.alfredworkflow` — confirm that file now
exists and is non-empty.

Sanity-check the bundle actually carries the new version and a single
binary:

```bash
unzip -p dist/alfred-emoji-rs.alfredworkflow info.plist | plutil -extract version raw -
unzip -l dist/alfred-emoji-rs.alfredworkflow
```

### 5. Commit the version bump

```bash
git add Cargo.toml Cargo.lock info.plist
git commit -m "Release vX.Y.Z"
```

Don't fold in unrelated changes — if `git status` shows anything else
staged or modified, stop and ask.

### 6. Confirm before the visible, hard-to-reverse steps

Everything from here on pushes to `origin/main` and publishes a public
GitHub release — show the user the version, the commit, and ask for
explicit go-ahead before tagging/pushing/publishing. A prior "yes, cut a
release" does not by itself authorize silently repeating this for a
future version.

### 7. Tag, push, and publish

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main --follow-tags
gh release create vX.Y.Z dist/alfred-emoji-rs.alfredworkflow \
  --title "vX.Y.Z" \
  --generate-notes
```

`--generate-notes` builds notes from the commits since the last tag,
which matches this repo's commit style (see `git log`). If the user wants
custom release notes instead, ask what to say rather than guessing.

### 8. Report

Give the user the release URL (`gh release view vX.Y.Z --web` prints it,
or read it from the `gh release create` output) and confirm the asset
attached is the one built in step 4, not a stale `dist/` artifact.
