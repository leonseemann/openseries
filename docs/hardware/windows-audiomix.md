# Windows AudioMix (M4a session mixer) — test guide and verification log

Goal: verify the session mixer ([ADR 0004](../adr/0004-windows-audio-strategy.md),
[audio-engine.md §5](../architecture/audio-engine.md)) on Windows with the Nova Pro Wireless.
Quit SteelSeries GG first (it also changes app volumes and owns the ChatMix dial).

## Setup

```powershell
cargo build --release -p ss-app -p winaudio-spike
.\target\release\openseries.exe
```

Open the Windows volume mixer (*Settings → System → Sound → Volume mixer*) next to the app to see
the per-app sliders move.

## Checklist

Run on 2026-11-14: Windows 11 Pro (build 26300), Nova Pro Wireless, GG not running. **All checks passed.**

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | fmt, clippy, tests | green | ✅ Observed 2026-11-14 |
| 2 | Session volume is linear (**Inferred**): `winspike capture --name chrome.exe --silence half` | loopback ≈ 6 dB lower in phase 2 | ✅ Observed 2026-11-14: with volume 0.5 the loopback peak was 6.0–6.3 dB below the session meter in 8 of 9 seconds (once 7.3 dB, a quiet passage during the ramp); linear |
| 3 | AudioMix page | apps listed by exe name (`chrome.exe`, `spotify.exe`), ▶ for playing ones; channel DSP replaced by the Windows note | ✅ Observed 2026-11-14 |
| 4 | AudioMix on, assign e.g. Chrome → Game, Spotify/Discord → Chat | nothing audible changes at 100 %; app sliders in the Windows mixer unchanged | ✅ Observed 2026-11-14 |
| 5 | Channel volume / mute (Game 50 %, then Mute) | only the Game app gets quieter / silent; its mixer slider follows | ✅ Observed 2026-11-14 |
| 6 | ChatMix: app slider, Ctrl+Alt+←/→, base-station dial | Game and Chat apps change in opposite directions | ✅ Observed 2026-11-14 |
| 7 | Master volume / mute | all apps follow | ✅ Observed 2026-11-14 |
| 8 | Change an app's slider in the Windows mixer while AudioMix runs | the new level is kept (not snapped back) | ✅ Observed 2026-11-14 |
| 9 | App started while AudioMix runs (e.g. open a new player) | lands on its channel / Media at the right level within ~2 s | ✅ Observed 2026-11-14 |
| 10 | AudioMix off, and Quit | every app slider back to where it was before | ✅ Observed 2026-11-14 |
| 11 | Crash safety: Game at 20 %, kill `openseries.exe` in Task Manager, restart the app | app volumes restored on start; `%APPDATA%\OpenSeries\session-volumes.txt` gone | ✅ Observed 2026-11-14 |
| 12 | Virtual microphone switch | "needs the OpenSeries audio driver (milestone M4b)", nothing crashes | ✅ Observed 2026-11-14 |

## Smoke test without the owner (2026-11-14, agent)

Isolated config (`APPDATA` pointed to a scratch folder, `OPENSERIES_NO_HEADSET=1`), AudioMix on
with all faders at 100 % (so no audible change): the session mixer started, wrote the restore file
(here the Spotify session) and kept it after a hard kill; the next start with AudioMix off restored
it and deleted the file. Session volumes before and after (`winspike sessions`) were identical.
