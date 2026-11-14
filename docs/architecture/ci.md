# Continuous integration

Workflow: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml). It runs the checks from
AGENTS.md: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace` (with `--locked`).

| Job | Runner | When | Why |
|---|---|---|---|
| rustfmt | ubuntu-latest | every push / PR | cheapest runner, no compile |
| clippy + test (Windows) | windows-latest | every push / PR | the platform nobody builds locally (owner develops on a Mac) |
| clippy + test (macOS) | macos-latest | PRs and manual runs | macOS minutes are billed 10× on a private repo; the same checks run locally before each commit |

Build time is kept low by:

- `Swatinem/rust-cache` (per OS, keyed on `Cargo.lock` and the toolchain): `~/.cargo` registry/git
  and `target/`; `cache-on-failure` so a failing run still warms the cache.
- One job per OS for clippy **and** tests, building the test binaries first so both reuse the same
  compiled dependencies.
- `CARGO_INCREMENTAL=0` and no debug info in dev/test profiles (smaller cache, faster links).
- `paths-ignore` for docs-only commits; `concurrency` cancels superseded runs.

Run macOS on demand: GitHub → Actions → CI → "Run workflow", or `gh workflow run CI`.
Linux is not built (the app's tray/HID dependencies need system packages and Linux is not a
target platform).
