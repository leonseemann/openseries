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
# Microphone page
mic-title = Microphone
mic-subtitle = A cleaner voice for Discord, Zoom and OBS: noise suppression, gate, stabilizer and EQ.
mic-unavailable-title = Not available on this computer
mic-unavailable-body = The processed "OpenSeries Microphone" needs a virtual audio device. It is available on macOS. On Windows it would require a paid, signed audio driver, and OpenSeries stays free — so this page is empty here.
mic-unavailable-headset-hint = Microphone volume, sidetone and the mute light are on the Headset page.
mic-virtual = Virtual microphone
mic-virtual-hint = Select "OpenSeries Microphone" as the input in your apps.
mic-install-driver-hint = Install the microphone driver once: Settings → Show getting started → Install driver.
mic-source = Source
mic-input = Microphone
mic-input-hint = Usually your headset.
mic-as-default = Use as system input
mic-as-default-hint = Make "OpenSeries Microphone" the default input while it runs.
mic-default-input-error = Default input: { $error }
mic-backup = Backup microphones
mic-processing = Processing
mic-denoise = Noise suppression
mic-denoise-hint = Removes fans, keyboard hum and background noise.
mic-stabilizer = Volume stabilizer
mic-stabilizer-hint = Evens out quiet and loud speech.
mic-gain = Output gain
mic-gain-hint = Louder or quieter overall.
mic-presets = Presets
mic-gate = Noise gate
mic-gate-hint = Silence between sentences. Automatic adapts to your room.
mic-gate-off = Off
mic-gate-automatic = Automatic
mic-gate-manual = Manual
mic-gate-threshold = Threshold
mic-gate-threshold-hint = Quieter sounds are muted.
mic-gate-sensitivity = Sensitivity
mic-gate-sensitivity-hint = How far above the room noise your voice must be.

# Window frame: sidebar and banners above every page
nav-overview = Overview
nav-device = Device
nav-audiomix = AudioMix
nav-mic = Microphone
nav-settings = Settings
nav-devices-heading = DEVICES
nav-no-devices = None connected
# $state is common-on, common-off, common-paused or common-error.
nav-status-mic = Microphone · { $state }
nav-status-audiomix = AudioMix · { $state }
nav-no-device-connected = No device connected
nav-device-reading = { $device } · reading…
nav-device-off = { $device } · off
nav-device-battery = { $device } · { $percent } %
nav-device-connected = { $device } · connected
nav-paused-banner = OpenSeries is paused: no supported device is connected. Until one is back, OpenSeries does not touch app audio, microphones or hotkeys, so other devices work undisturbed.
nav-update-available = OpenSeries { $version } is available.
nav-install-update = Install update
nav-update-details = Details
nav-hotkeys-unavailable = Hotkeys unavailable: { $error }

# Shared widgets
common-off = Off
common-on = On
common-paused = Paused
common-error = Error
common-paused-reason = Paused: { $reason }
common-system-default = System default
common-not-connected = { $device } (not connected)
common-add-backup = Add backup…

# Desktop notifications ($hub is what the computer is plugged into, e.g. "base station")
alert-linked = { $device } connected
alert-unlinked = { $device } disconnected (off or out of range)
alert-gone = { $device }: { $hub } disconnected — unplugged or switched to another computer
alert-back = { $device }: { $hub } connected
alert-battery-low = { $device }: battery low (≈{ $percent } %)
alert-service-failed = { $service } stopped: { $message }
alert-service-audiomix = AudioMix
alert-service-virtual-mic = Virtual microphone

# Messages from the operating system adapters
platform-autostart-failed = Start at login: { $error }
platform-autostart-unavailable = Start at login is not available on this operating system.
platform-usb-reset-unavailable = Reconnect USB is not available on this operating system yet.
platform-crackling-watch-unavailable = The crackling watch is not available on this operating system yet.
platform-mic-driver-unavailable = There is no virtual microphone driver for this operating system yet.
platform-update-unavailable = There are no update packages for this operating system.
platform-driver-install-cancelled = Installation cancelled.
# -- end of microphone section --
