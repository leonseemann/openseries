# Windows AudioMix (M4a session mixer) — test guide and verification log

Goal: verify the session mixer ([ADR 0004](../adr/0004-windows-audio-strategy.md),
[audio-engine.md §5](../architecture/audio-engine.md)) on Windows with the Nova Pro Wireless.
Quit SteelSeries GG first (it also changes app volumes and owns the ChatMix dial).

## Setup

```powershell
cargo build --release -p ss-app -p winaudio-spike
.\target\release\stealseries.exe
```

Open the Windows volume mixer (*Settings → System → Sound → Volume mixer*) next to the app to see
the per-app sliders move.

## Checklist

| # | Check | Expected | Result |
|---|-------|----------|--------|
| 1 | fmt, clippy, tests | green | ✅ Observed 2026-11-14 |
| 2 | Session volume is linear (**Inferred**): `winspike capture --name chrome.exe --silence half` | loopback ≈ 6 dB lower in phase 2 | ⏳ |
| 3 | AudioMix page | apps listed by exe name (`chrome.exe`, `spotify.exe`), ▶ for playing ones; channel DSP replaced by the Windows note | ⏳ |
| 4 | AudioMix on, assign e.g. Chrome → Game, Spotify/Discord → Chat | nothing audible changes at 100 %; app sliders in the Windows mixer unchanged | ⏳ |
| 5 | Channel volume / mute (Game 50 %, then Mute) | only the Game app gets quieter / silent; its mixer slider follows | ⏳ |
| 6 | ChatMix: app slider, Ctrl+Alt+←/→, base-station dial | Game and Chat apps change in opposite directions | ⏳ |
| 7 | Master volume / mute | all apps follow | ⏳ |
| 8 | Change an app's slider in the Windows mixer while AudioMix runs | the new level is kept (not snapped back) | ⏳ |
| 9 | App started while AudioMix runs (e.g. open a new player) | lands on its channel / Media at the right level within ~2 s | ⏳ |
| 10 | AudioMix off, and Quit | every app slider back to where it was before | ⏳ |
| 11 | Crash safety: Game at 20 %, kill `stealseries.exe` in Task Manager, restart the app | app volumes restored on start; `%APPDATA%\StealSeries\session-volumes.txt` gone | ⏳ |
| 12 | Virtual microphone switch | "needs the StealSeries audio driver (milestone M4b)", nothing crashes | ⏳ |
