# Hardware verification log — Arctis Nova Pro Wireless

Protocol facts start as **Inferred** (from [prior-art §1](../research/prior-art.md)) and become
**Observed** once verified on the owner's device with `ssctl`. Record every check here.

Test setup: base station PID `0x12E0`, Windows 11, 2026-11-14. SteelSeries GG/Sonar was running
in parallel (it shares the device; replies were not disturbed).

## Results

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Enumeration: interface 4, two vendor collections | `ssctl list` (no data sent) | `MI_04&Col01` usage page `0xFFC0`, `MI_04&Col02` usage page `0xFF00` | ✅ Observed |
| Status block `06 B0` | `ssctl raw status` | Layout as documented; `[11]` = mic-LED brightness confirmed | ✅ Observed |
| Battery `06 B7` | `ssctl raw battery` | `[2]` headset 4 (=50 %), `[3]` = 8 matches `B0[7]` spare battery → `[3]` is the **spare battery** (resolves the conflict) | ✅ Observed |
| Settings block `06 20` | `ssctl raw settings` | `[3]` volume attenuation, `[6]` EQ preset, `[7..16]` bands, `[17]` mic volume, `[18]` sidetone, `[20]/[21]` ChatMix | ✅ Observed |
| Display block `06 80` | `ssctl raw display` | `[3]` OLED brightness, `[7]` mixer icon (1 while Sonar runs) | ✅ Observed |
| Noise control `06 BD` | `set noise-control anc/transparency/off` | Read back via `B0[10]` | ✅ Observed |
| Transparency `06 B9` | `set transparency 5` | Read back via `B0[8]` | ✅ Observed |
| Auto-off `06 C1` | `set auto-off 60` | Read back via `B0[12]` | ✅ Observed |
| Sidetone `06 39` | `set sidetone medium` | Read back via `20[18]` | ✅ Observed |
| Mic volume `06 37` | `set mic-volume 7` | Read back via `20[17]` | ✅ Observed |
| EQ preset `06 2E` | `set eq-preset bass-boost` | Read back via `20[6]` | ✅ Observed |
| Custom EQ `06 33` | `set eq -- -2 0 1.5 2 0 0 0 1 2 3` | Read back exactly incl. 0.5 dB step | ✅ Observed |
| OLED brightness `06 85` | `set oled-brightness 4` | Read back via `80[3]` | ✅ Observed |
| Mic-mute LED `06 BF` | `set mic-led 3` | Read back via `B0[11]` | ✅ Observed |
| Volume dial events | `ssctl watch` | `07 25 <attenuation>`, 0 = loudest … 56 = silent | ✅ Observed |
| ChatMix dial events | `ssctl watch` (dial already enabled by Sonar) | `07 45 <game> <chat>`, steps of ~8 %, centre 100/100 | ✅ Observed |
| ChatMix dial enable `06 49` | `watch --chatmix-dial` | not tested (would interfere with running Sonar) | ⏳ pending |
| Save to flash `06 09` | `--save` | not tested yet (all test values were restored instead) | ⏳ pending |
| ANC button event | `ssctl watch`, press ANC button | `07 BD <mode>` with the same 0/1/2 encoding as the write command; observed sequence 2→0→2 (button cycle is ANC ↔ off on this unit; whether transparency is in the cycle depends on a setting — Unknown) | ✅ Observed (new, not in prior art) |
| Mic mute switch event | `ssctl watch`, toggle mute | `07 BB 01` = muted, `07 BB 00` = unmuted | ✅ Observed (new, not in prior art) |
| Hardware EQ band centre frequencies (Unknown) | sweep / listening test | | ⏳ pending |

All values changed during testing were restored to their previous state.

## How to run a check

```bash
cargo run -p ssctl -- status
```

```bash
cargo run -p ssctl -- watch
```

Settings change only in RAM unless `--save` is passed. Factory reset and firmware commands are not
implemented on purpose.
