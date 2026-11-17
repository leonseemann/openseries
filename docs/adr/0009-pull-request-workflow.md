# ADR 0009 — Changes land on `main` through pull requests

- **Status:** accepted (2026-11-17, owner)

## Context

Since the repository became public (ADR 0006) it had no protection: anyone with write access could
push to, force-push or delete `main`, merged branches piled up, and nothing forced the CI checks to
pass before a change landed. Development so far pushed small commits straight to `main`.

## Decision

Repository rules are GitHub settings and rulesets, recorded here because they are not in the tree.

**`main` (ruleset "main")**

- No direct pushes: every change is a pull request. No approving review is required (single
  maintainer; GitHub does not let authors approve their own PRs), but review conversations must be
  resolved before merging.
- Required status checks from [CI](../architecture/ci.md): `rustfmt`, `clippy + test (Windows)`,
  `clippy + test (macOS)`. Branches need not be up to date with `main` first (fewer reruns of the
  slow macOS job).
- Linear history; no force pushes; `main` cannot be deleted.
- Bypass: repository admins, **only through a pull request** (e.g. to merge with a red check in an
  emergency); direct pushes stay blocked for them too.

**Release tags (ruleset "release tags", `refs/tags/v*`)**

- Tags cannot be deleted or moved once pushed: installed apps verify updates against published
  releases. Admins may bypass (e.g. to delete a tag whose release workflow failed before
  publishing).

**Repository settings**

- Merge methods: squash and rebase; no merge commits. Squash commits take the PR title, so PR
  titles follow Conventional Commits.
- Head branches are deleted automatically after the merge; "Update branch" and auto-merge are on.
- Security: Dependabot alerts and security updates, secret scanning with push protection, private
  vulnerability reporting ([SECURITY.md](../../SECURITY.md)).

**In the tree:** [CONTRIBUTING.md](../../CONTRIBUTING.md), a pull request template, issue forms,
and Dependabot version updates for GitHub Actions (`.github/`).

## Consequences

- Work happens on short-lived branches (`feat/…`, `fix/…`, `docs/…`); "commit small and often"
  now applies within the branch, and a PR collects a coherent change.
- CI runs for every pull request, also docs-only ones, otherwise the required checks would never
  report and the PR could not be merged. Pushes to `main` keep skipping docs-only changes.
- Release commits (`chore: release X.Y.Z`) go through a PR as well; the tag is pushed after the
  merge, on the merged commit.
- Cargo dependency version bumps are not automated: each one needs the third-party licence list
  regenerated (AGENTS.md), so they stay manual. Security updates still open PRs.
