# Release Skill

Publish a new version of rustfs-mimalloc to crates.io and create a GitHub Release.

## Trigger

Activate when the user says "release", "publish", "bump version", "tag", or similar.

## Pre-check

```bash
git status --short
git tag --sort=-v:refname | head -5
```

## Workflow

### 1. Determine Version

Ask the user for the target version (e.g. `0.1.0`). If not specified, increment the patch version of the latest tag.

### 2. Version Consistency Check

Verify that all three Cargo.toml files have matching versions:

```bash
grep '^version' Cargo.toml rustfs-mimalloc-sys/Cargo.toml rustfs-mimalloc/Cargo.toml
```

If inconsistent, use Edit to update all to the target version. Also update the sys crate dependency version in `rustfs-mimalloc/Cargo.toml`:

```toml
rustfs-mimalloc-sys = { path = "../rustfs-mimalloc-sys", version = "<NEW_VERSION>" }
```

### 3. Local Validation

Run these checks in order. Stop and report on first failure:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --features secure,debug,win_direct_tls
cargo doc --no-deps
```

### 4. Commit Version Bump

```bash
git add -A
git commit -m "release: v<VERSION>"
```

### 5. Tag and Push

```bash
git tag v<VERSION>
git push origin main --tags
```

After push, GitHub Actions automatically runs:
- 3-platform test matrix (Linux / macOS / Windows)
- Lint checks
- Publish to crates.io (sys crate first, 30s delay, then wrapper)
- Create GitHub Release with auto-generated changelog

### 6. Confirm

Tell the user:
- CI has been triggered — view progress in Actions tab
- crates.io: `https://crates.io/crates/rustfs-mimalloc/<VERSION>`
- GitHub Release: `https://github.com/houseme/rustfs-mimalloc/releases/tag/v<VERSION>`

## Manual Trigger (Alternative)

To trigger a release for an existing tag, or with dry run:

```bash
gh workflow run release.yml -f version=<VERSION> -f dry_run=true
```

## Rollback

If something goes wrong after publish:

```bash
cargo yank --vers <VERSION> -p rustfs-mimalloc
cargo yank --vers <VERSION> -p rustfs-mimalloc-sys
```

## Notes

- Version follows SemVer: `MAJOR.MINOR.PATCH[-PRERELEASE]`
- crates.io does not allow re-publishing the same version
- Requires repository Secret `CARGO_REGISTRY_TOKEN`
- Pre-release versions (containing `-`) are auto-tagged as pre-release on GitHub
