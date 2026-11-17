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
# -- end of AudioMix section --

## Settings, onboarding, hotkeys, updates (ui/settings.rs, ui/onboarding.rs, hotkeys.rs, update.rs)
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
