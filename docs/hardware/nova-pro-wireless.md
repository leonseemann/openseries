# Hardware verification log — Arctis Nova Pro Wireless

Protocol facts start as **Inferred** (from [prior-art §1](../research/prior-art.md)) and become
**Observed** once verified on the owner's device with `ssctl`. Record every check here.

| Date | Check | Command | Result | Status |
|------|-------|---------|--------|--------|
| 2026-11-14 | Enumeration on Windows 11: PID, interface 4, two vendor collections | `ssctl list` (no data sent) | PID `0x12E0`; `MI_04&Col01` usage page `0xFFC0`, `MI_04&Col02` usage page `0xFF00` | ✅ Observed |
| | Status block `06 B0` | `ssctl status` | | ⏳ pending |
| | Battery `06 B7` incl. spare battery slot (resolves `B7[3]` conflict) | `ssctl battery` while swapping batteries | | ⏳ pending |
| | Noise control `06 BD` | `ssctl set noise-control anc` | | ⏳ pending |
| | Transparency `06 B9` | `ssctl set transparency 5` | | ⏳ pending |
| | Sidetone `06 39` | `ssctl set sidetone medium` | | ⏳ pending |
| | Mic volume `06 37` | `ssctl set mic-volume 8` | | ⏳ pending |
| | EQ preset `06 2E` / custom EQ `06 33` | `ssctl set eq-preset bass-boost`, `ssctl set eq …` | | ⏳ pending |
| | ChatMix dial `06 49` + events `07 45` | `ssctl watch --chatmix-dial` | | ⏳ pending |
| | Auto-off `06 C1` | `ssctl set auto-off 30` | | ⏳ pending |
| | OLED brightness `06 85` | `ssctl set oled-brightness 3` | | ⏳ pending |
| | Mic-mute LED `06 BF` | `ssctl set mic-led 5` | | ⏳ pending |
| | Save to flash `06 09` | `--save` flag | | ⏳ pending |
| | ANC button event id (Unknown) | `ssctl watch`, press ANC button | | ⏳ pending |
| | Hardware EQ band centre frequencies (Unknown) | listening test / sweep | | ⏳ pending |

## How to run a check

```bash
cargo run -p ssctl -- status
```

Settings are changed only in RAM unless `--save` is passed. Factory reset and firmware commands
are not implemented on purpose.
