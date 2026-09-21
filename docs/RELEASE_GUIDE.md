# Nimbus Release Guide

Step-by-step process for cutting a new Nimbus release. Follow this in order -
the automated workflow depends on a few things being correct _before_ you
tag (version number, changelog heading), and it can't fix those for you
after the fact without a real code change.

Quick-reference checklist is at the bottom if you just need the commands.

---

## 1. Decide the version number

Nimbus is pre-1.0, so semver works a little differently than you might
expect:

- **Patch (`0.1.0` → `0.1.1`)** — bug fixes only, no new features, no
  behavior changes.
- **Minor (`0.1.0` → `0.2.0`)** — new features, or _any_ breaking change.
  Below 1.0.0, breaking changes bump the minor version, not the major —
  that's the semver spec's own rule for `0.x` releases, not a Nimbus
  convention.
- **Major (`1.0.0`)** — reserved for when Nimbus is stable enough to
  commit to compatibility guarantees. Not relevant yet.

In practice, right now: finishing GCP support is a minor bump (`0.2.0`),
a fix to something already shipped is a patch (`0.1.x`).

Never reuse or move a tag that's already been pushed and has a real,
published release attached — see the "If something goes wrong" section.

## 2. Update `Cargo.toml`

```toml
[package]
version = "0.2.0"   # matches the tag you're about to create, no "v" prefix
```

The release workflow doesn't read this file, but it should always match
the tag — a mismatch here is confusing later when someone's binary
reports a version that doesn't match how it was tagged.

## 3. Update `CHANGELOG.md`

Move everything currently sitting under `## [Unreleased]` into a new
dated section, and leave `## [Unreleased]` empty at the top for whatever
comes next:

```markdown
## [Unreleased]

## [0.2.0] - 2026-10-03

### Added

- GCP provider: GCE, Cloud SQL, Cloud Storage listing
- All Clouds tab showing combined AWS + GCP resources

### Fixed

- ...
```

**The heading must be exactly `## [x.y.z]`, matching the tag without its
`v` prefix.** The release workflow extracts this section by that exact
heading — `## [0.2.0]` for tag `v0.2.0`. If it doesn't match, the release
job fails outright rather than publishing an empty release, which is
intentional, but it means you don't get a working release until you fix
it and re-tag.

## 4. Run the checks locally before tagging

CI runs these too, but a failure there means a wasted 60-90 minutes
across three runners (Linux, Windows, macOS) before you find out. Catch
it locally first:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

If you touched `Cargo.toml` dependencies, also regenerate the lockfile —
CI builds with `--locked` and will fail if it's stale:

```bash
cargo build --release
git status   # confirm Cargo.lock changed if dependencies changed
```

## 5. Commit and push to `main`

```bash
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "chore: prepare v0.2.0 release"
git push origin main
```

Tag _after_ this lands on `main`, not before — the tag needs to point at
a commit that already has the right version number and changelog section,
since that's what the workflow reads.

## 6. Tag and push

```bash
git tag -a v0.2.0 -m "Release v0.2.0: GCP support"
git push origin v0.2.0
```

Pushing the tag triggers `.github/workflows/release.yml`:
`checks` → `build` (Linux + Windows) and `build-macos` (arm64 native +
x86_64 cross-compiled) in parallel → `release` (extracts changelog
section, publishes to GitHub Releases with all binaries attached).

## 7. Watch the run and verify the release

```bash
gh run list --workflow=release.yml
gh run watch          # follow the current run
```

Once it finishes, check the release page itself:

- All expected assets are attached: Linux `.tar.gz`, Windows `.zip`,
  macOS arm64 `.tar.gz`, macOS x86_64 `.tar.gz` — each with a matching
  `.sha256` file
- Release notes show your changelog section, not a blank body or a raw
  commit list
- Grab one binary and sanity-check it runs (`--version` or just launch
  it) before telling anyone the release is out

## If something goes wrong

**Workflow fails before publishing anything** (e.g. `checks` fails,
changelog heading doesn't match, a build target breaks) — no release was
created, so just fix it, commit, delete and re-push the tag:

```bash
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0
# fix the problem, commit, push to main
git tag -a v0.2.0 -m "Release v0.2.0: GCP support"
git push origin v0.2.0
```

Safe to do freely at this stage — nothing was published, so nobody has a
stale reference to invalidate.

**Workflow succeeds and a release is published, but something's wrong
with it that isn't a code problem** (typo in notes, wrong asset name) —
edit the release directly, don't retag:

```bash
gh release edit v0.2.0 --notes-file CHANGELOG_EXCERPT.md
gh release upload v0.2.0 corrected-asset.tar.gz --clobber
```

**A real bug shipped in the binaries themselves** — don't retag or
delete the release. Cut a new patch version instead (`v0.2.1`) with the
fix. A published release is a historical record; people may have already
downloaded it, and moving a tag out from under them silently breaks
anything pinned to it.

---

## Quick reference

```bash
# 1. Bump version
$EDITOR Cargo.toml                 # version = "x.y.z"

# 2. Update changelog
$EDITOR CHANGELOG.md               # move Unreleased -> ## [x.y.z] - date

# 3. Verify locally
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release

# 4. Commit and push
git add Cargo.toml CHANGELOG.md Cargo.lock
git commit -m "chore: prepare vx.y.z release"
git push origin main

# 5. Tag and push
git tag -a vx.y.z -m "Release vx.y.z: <summary>"
git push origin vx.y.z

# 6. Watch and verify
gh run watch
gh release view vx.y.z
```
