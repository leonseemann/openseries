# Contributing to OpenSeries

Thanks for helping! Bug reports, hardware test results and pull requests are welcome.

## Before you start

- Read [AGENTS.md](AGENTS.md): it holds the engineering rules (clean room, English only,
  architecture, safety rules) for humans and AI agents alike.
- For anything larger than a small fix, open an issue first so we can agree on the approach.
  Non-trivial changes start with a design note in `docs/architecture/` or an ADR in `docs/adr/`.
- **Clean room:** never contribute code, binaries, firmware, icons, sounds or other assets taken
  from SteelSeries software.

## Workflow

1. Fork the repository (maintainers: create a branch) named after the change, e.g.
   `feat/arctis-sidetone`, `fix/tray-crash`, `docs/release-notes`.
2. Commit small and often using [Conventional Commits](https://www.conventionalcommits.org/)
   (`feat(audio-mix): …`, `fix: …`, `docs: …`).
3. Run the checks before every commit:

   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

   After changing dependencies, regenerate the licence list: `python3 scripts/third_party_licenses.py`.
4. Update the documentation the change touches; a change is not done without it.
5. Open a pull request against `main` and fill in the template. Use a Conventional Commit as the
   PR title — it becomes the commit message when the PR is squash-merged.

`main` is protected: changes land only through pull requests with passing CI (rustfmt, clippy and
tests on Windows and macOS) and resolved review conversations, with linear history. Merged branches
are deleted automatically. Details: [ADR 0009](docs/adr/0009-pull-request-workflow.md).

## Hardware

Pull requests are tested without devices in CI. If your change talks to a headset, say which
device and OS you tested it on; the test protocols are in [`docs/hardware/`](docs/hardware/).

## Security issues

Do not open a public issue; see [SECURITY.md](SECURITY.md).

## Licence

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project shall be dual licensed under Apache-2.0 or MIT, without any additional terms or
conditions (see the [README](README.md#license)).
