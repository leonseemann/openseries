# OpenSeries — English (source language and fallback for every other language).
# Translating: copy this file to i18n/<language>/openseries.ftl and translate the values;
# see docs/architecture/i18n.md. Keep ids, { $arguments } and the structure unchanged.

# The language's name in itself, shown in Settings → Language.
language-name = English

## Tray menu

tray-audiomix = AudioMix
tray-virtual-mic = Virtual microphone
tray-reconnect-usb = Reconnect USB (fixes crackling)
tray-check-updates = Check for updates
tray-open = Open OpenSeries…
tray-quit = Quit OpenSeries
tray-paused = Paused · no supported device connected
tray-no-device = No device connected
tray-device-off = { $device } · off
tray-device-connected = { $device } connected
tray-device-battery = { $device } · ≈{ $percent } %
tray-checking-updates = Checking for updates…
tray-downloading-update = Downloading OpenSeries { $version }…
tray-installing-update = Installing OpenSeries { $version }…
tray-install-update = Install update { $version }…

## Settings → General → Language

settings-language = Language
settings-language-hint = Add more languages as i18n/<code>/openseries.ftl in the config folder.
settings-language-system = System default

## Device pages and overview (ui/device.rs, ui/overview.rs)
# -- end of device section --

## AudioMix and presets (ui/audiomix.rs, ui/presets.rs, audio/)
mix-subtitle = Sort your apps into Game, Chat, Media and Aux and control each group separately.
mix-switch-hint-dsp = Routes app audio through OpenSeries. When off, OpenSeries does not touch system audio at all — use that when another audio tool should handle your apps.
mix-switch-hint-volumes = Sets each app's volume by its channel and balances Game and Chat. When off, OpenSeries does not touch app volumes.
mix-balance-title = Game/Chat balance
mix-balance-hint = Also follows the dial on your device and the balance hotkeys.
mix-balance-split = Game { $game } %  ·  Chat { $chat } %
mix-balance-centre = Centre
mix-output-title = Output
mix-play-through = Play through
mix-play-through-hint = Usually your headset.
mix-backup-outputs = Backup outputs
mix-backup-outputs-hint = Used in this order when the output above is missing (also for the microphone's backups).
mix-backup-when-headset-off = Use backups while the headset is off
mix-channels-title = Channels
mix-channels-windows-hint = On Windows, channels set the volume of their apps. Equalizer, boosts and Smart Volume per channel are available on macOS.
mix-all-channels = All channels
mix-channel-sound = { $channel } sound
mix-smart-volume-hint = Evens out loud and quiet passages.
mix-smart-volume-off = Off
mix-smart-volume-soft = Soft
mix-smart-volume-balanced = Balanced
mix-smart-volume-strong = Strong
mix-boost-bass = Bass
mix-boost-treble = Treble
mix-boost-clarity = Voice clarity
mix-boost-volume = Volume boost
mix-eq = Equalizer
mix-eq-flat = Flat
mix-apps-title = Apps
mix-show-idle = Show apps that are not playing
mix-show-idle-hint = Apps you have not assigned play on Media.
mix-no-apps = No app is playing sound right now. Start some audio and it appears here.
mix-app-playing = Playing
mix-app-idle = Idle
mix-app-not-running = Not running
mix-app-unassigned = not assigned — plays on Media
mix-reset = Reset
mix-reset-hint = Back to the automatic channel: { $channel }
audio-source-own = your choice
audio-source-catalogue = built-in list
audio-source-game-library = game library
audio-source-games-category = games category
audio-paused = no supported device is connected
audio-conflict = { $count ->
    [one] { $tools } is running and also captures app audio
   *[other] { $tools } are running and also capture app audio
}
audio-not-supported = not supported on this operating system
audio-output-missing = the selected output device is not connected
audio-mic-missing = the selected microphone is not connected
audio-mic-is-virtual = the system input is OpenSeries Microphone itself; choose the headset as source
audio-mic-macos-only = The virtual microphone is available on macOS only (Windows would need a signed audio driver)
audio-unsupported-mix = AudioMix is not available on this operating system yet
audio-unsupported-mic = The virtual microphone is not available on this operating system yet
presets-label = Preset
presets-none = no presets yet
presets-delete = Delete
presets-export = Export…
presets-import = Import…
presets-name-hint = name
presets-save = Save current
presets-applied = applied "{ $name }"
presets-deleted = deleted "{ $name }"
presets-imported-pick = imported "{ $name }" — pick it to apply
presets-imported-as = imported "{ $name }" as a { $kind } preset
presets-import-failed = import failed: { $error }
presets-replaced = replaced "{ $name }"
presets-saved = saved "{ $name }"
presets-nothing-to-save = nothing to save yet
presets-export-cancelled = export cancelled
presets-exported = exported to { $path }
presets-export-failed = export failed: { $error }
presets-file-filter = OpenSeries preset
presets-import-filter = Preset (OpenSeries or Sonar JSON)
presets-device-eq-elsewhere = this is a device EQ preset; import it on the device's page
presets-kind-device-eq = device EQ
presets-kind-channel = channel
presets-kind-mic = microphone
# -- end of AudioMix section --

## Settings, onboarding, hotkeys, updates (ui/settings.rs, ui/onboarding.rs, hotkeys.rs, update.rs)
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
