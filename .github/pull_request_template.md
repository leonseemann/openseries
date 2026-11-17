<!-- PR title: a Conventional Commit, e.g. "feat(audio-mix): per-app reset" (it becomes the squash commit). -->

## What and why

<!-- What does this change, and why? Link the issue: "Closes #123". -->

## Testing

<!-- How was it tested? For device or audio changes: which device(s) and OS. -->

## Checklist

- [ ] `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass
- [ ] Docs updated (`docs/`, README) where the change affects them
- [ ] Licence list regenerated if dependencies changed (`python3 scripts/third_party_licenses.py`)
- [ ] Clean room: no code or assets taken from SteelSeries software
