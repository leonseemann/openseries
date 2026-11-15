# ADR 0003 — License: MIT OR Apache-2.0

- **Status:** Accepted (2026-11-14)
- **Context docs:** [prior-art §7](../research/prior-art.md)

## Decision

All OpenSeries code is dual-licensed under **MIT OR Apache-2.0** (the Rust ecosystem default).
Files derived from Microsoft's SysVAD sample (possible future Windows driver) would keep their
**MS-PL** license in a separate directory.

## Reasons

- Every planned dependency (`hidapi`, `objc2-*`, `wasapi`, `windows`, `rtrb`, `hound`, …) is
  compatible.
- GPL projects in the prior-art list (HeadsetControl, BlackHole, Background Music, …) are used as
  **references only**; the clean-room rule already forbids copying their code, so GPL would gain us
  nothing while blocking MS-PL driver code and limiting reuse of our protocol crates.

## Consequences

- `license = "MIT OR Apache-2.0"` in the workspace manifest; `LICENSE-MIT` and `LICENSE-APACHE` at
  the repository root.
- Contributions are accepted under the same dual license (see README).
