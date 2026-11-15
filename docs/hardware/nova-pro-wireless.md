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
| ChatMix set `06 45 <game> <chat>` (new, not in prior art; inferred from write opcode = event opcode) | `ssctl set chatmix 100 50` on macOS, OpenSeries not running (dial not handed over) | Settings block `20[20]/[21]` read back `64 32` (game 100, chat 50); restored with `100 100` → `64 64`. With the dial handed over (OpenSeries AudioMix on), the app's slider/hotkeys update the base-station ChatMix display and the dial continues from the new position | ✅ Observed (2026-11-14, macOS) |
| ChatMix dial enable `06 49` | `watch --chatmix-dial` | not tested (would interfere with running Sonar) | ⏳ pending |
| Save to flash `06 09` | `--save` | not tested yet (all test values were restored instead) | ⏳ pending |
| ANC button event | `ssctl watch`, press ANC button | `07 BD <mode>` with the same 0/1/2 encoding as the write command; observed sequence 2→0→2 (button cycle is ANC ↔ off on this unit; whether transparency is in the cycle depends on a setting — Unknown) | ✅ Observed (new, not in prior art) |
| Mic mute switch event | `ssctl watch`, toggle mute | `07 BB 01` = muted, `07 BB 00` = unmuted | ✅ Observed (new, not in prior art) |
| Custom EQ is audible | Windows, 2026-11-14, music with vocals/cymbals: `set eq-preset flat` → `set eq -- 0 0 0 0 0 -10 -10 -10 -10 -10` (writes `06 33`, then selects Custom) → flat → `set eq-preset custom` only | Owner heard "muffled" exactly in both Custom phases: the curve is applied, bands run low → high, and re-selecting Custom applies the stored curve. +10 dB on bands 1–2 alone was hard to notice (very low bands, Inferred). Whether `06 33` alone applies live while Custom is active: Unknown (the app re-selects Custom after every write) | ✅ Observed |
| Firmware version `06 10` (prior art: ggoled, layout was undocumented) | `ssctl raw firmware`, `ssctl status` (Windows, 2026-11-14) | Reply args = four ASCII fields of 12 chars `NNNN.NNN.NNN`: `0000.003.082 0001.031.000 0002.002.001 0002.001.000`. GG's engine DB (`physical_devices.firmware_version`, read from a copy) shows the same unit as `0.3.82/1.31.0/2.2.1/2.1.0`, so the fields are decimal `major.minor.patch`. Which component each version belongs to is Unknown. USB descriptor: `bcdDevice` 0x0130, no serial number string, manufacturer "SteelSeries" | ✅ Observed |
| Factory reset `06 FD` (DEV-12) | not sent | Single source (HeadsetBatteryTray README, `prior-art.md` §1.3: "never send"). Unknown: whether it also clears the headset ↔ base-station pairing or the Bluetooth pairing, whether it reboots the base station, and what the defaults are. Not representable in `ss_nova_pro::Command` by design. Verification plan if the owner wants it: read and note every setting (`ssctl status`, `raw settings/display/firmware`), send `06 FD` once from a test build with the owner present, re-read, check the headset still links (re-pair procedure ready), then decide whether to expose it behind a confirmation | ⏳ blocked on owner decision |
| Switching the USB input (USB-1 ↔ USB-2) from a host | Windows, 2026-11-14: `ssctl watch` running while the owner switched the base station's input to the Mac (other USB port) and back | The inactive host loses the device **completely**: `watch` ended with a HID read error ("device not connected"), `ssctl status`/`list` found no base station; no event was reported before the switch. After switching back, `status` worked again at once. So the inactive host cannot ask the base station to come back. A host-side switch command is **Unknown**: GG offers no such feature (no UI string in `localization/*.json`, no field in the Nova Pro `configurations.settings`, `engine.md` §4.3), [nova-chatmix-linux](https://github.com/Dymstro/nova-chatmix-linux) and HeadsetControl document none; the per-device command bytes sit in GG's encrypted `.edevice` files, which we do not decrypt. Finding it would need probing unknown opcodes on the device (not done). Same result when switching **from the headset** (second run): no report of any kind (`watch` prints unrecognised reports too) before the HID read failed, so there is no `07 xx` event to derive a `06 xx` write from | ✅ Observed (disconnect); command Unknown |
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
