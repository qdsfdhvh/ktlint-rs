# Local Install Rules

Read before installing, updating, tagging, or releasing the `ktlint-rs` binary.

## Hard rule

The local `ktlint-rs` on PATH (`~/.cargo/bin/ktlint-rs`) must be installed
**only from a GitHub release** — never by copying a local build
(`target/release/ktlint-rs`) and never `cargo install` from a working tree.

- Official install: `scripts/install-release.sh [tag] [install-dir]`
  (downloads the per-platform asset from
  `github.com/qdsfdhvh/ktlint-rs/releases/download/<tag>/`).
- Local `target/release` builds are for development/testing only and must
  never replace the PATH binary.
- After install, verify: `ktlint-rs --version` must equal the released tag.

## Release procedure

1. Bump `Cargo.toml` and `Cargo.lock` (only the `ktlint-rs` package — do not
   blanket-replace version strings).
2. Merge the version PR with a green CI (fmt, check, tests, clippy, perf
   gates, mutation gates, spotless differential).
3. `git tag vX.Y.Z && git push origin vX.Y.Z` — the release workflow
   (`.github/workflows/release.yml`) builds and uploads linux/macOS/Windows
   binaries.
4. `gh release create vX.Y.Z --title ... --notes ...` if not auto-created.
5. Install locally via `scripts/install-release.sh vX.Y.Z`.
