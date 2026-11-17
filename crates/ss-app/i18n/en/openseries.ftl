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
device-title = Device
device-gone = The device is no longer connected.
# { $class } is a device class word such as "Headset".
device-subtitle = { $class } — settings stored on the device.
device-experimental = Experimental support: this device was not tested on hardware by the OpenSeries team. Settings may not work as described.
# { $hub } is where the device connects, e.g. "base station".
device-reading = Reading the { $hub }…
device-unlinked = The { $hub } is connected, but the { $class } is off or out of range. Settings stored in the { $hub } can still be changed.
device-none = No supported device connected
device-none-hint = Connect your device to this computer. The Arctis Nova Pro Wireless connects with its base station (on macOS the USB-1 port); if it is plugged in, it may be switched to the other USB input — switch it back on the base station.

device-group-noise-control = Noise control
device-group-microphone = Microphone
device-group-equalizer = Equalizer
device-group-power = Power and display
device-group-device = Device
device-group-more = More

device-noise-control-mode = Mode
device-noise-control-mode-hint = Also switchable with the button on the device.
device-transparency-strength = Transparency strength
device-transparency-strength-hint = How much of your surroundings you hear.
device-mic-mute = Status
device-mic-mute-hint = Muted with the button on the device.
device-mic-volume = Microphone volume
device-sidetone = Sidetone
device-sidetone-hint = Hear your own voice while you talk.
device-mic-led-brightness = Mute light brightness
device-mic-led-brightness-hint = The light on the microphone while it is muted.
device-eq-preset = Preset
device-eq-preset-hint = Stored on the device; works on everything it plays.
device-custom-curve = Custom curve
device-auto-off = Turn off when idle
device-auto-off-hint = Switches off after this long without sound.
device-display-brightness = Display brightness
device-firmware = Firmware
device-usb-replug = Reconnect USB
device-usb-replug-hint = Fixes crackling: like unplugging the cable (audio pauses for a second).
device-setting = Setting

device-battery = Battery
device-spare-battery = Spare battery
device-spare-battery-hint = The battery charging in the base station.
device-charge-discharging = On battery
device-charge-charging = Charging
device-charge-plugged = Plugged in
device-charge-unknown = Battery state unknown

device-unknown = Unknown
device-versions-unknown = unknown
device-versions-hint = Component versions as reported by the device. Which part each number belongs to may not be known.
device-reconnect = Reconnect
device-reconnect-unsupported = Not available on this operating system yet.
device-muted = Muted
device-live = Live
device-on = On
device-off = Off
# Bus of a device that is neither USB nor Bluetooth.
device-connection = Connection
device-release = release { $version }
device-no-curve = The device did not report a custom curve.
# { $min } and { $max } are signed gains such as "-10" and "+10".
device-curve-range = { $min } … { $max } dB per band, low to high
device-reset-flat = Reset to flat
device-band-low = Low
device-band-high = High

device-class-headset = Headset
device-class-keyboard = Keyboard
device-class-mouse = Mouse
device-class-other = Device
# The class words inside a sentence.
device-class-headset-inline = headset
device-class-keyboard-inline = keyboard
device-class-mouse-inline = mouse
device-class-other-inline = device

# Texts of the device drivers: device-<feature kind>-<option key or level name>, and
# device-hub-<hub name>. Drivers without a message here show their English text.
device-hub-base-station = base station
device-noise-control-off = Off
device-noise-control-transparency = Transparency
device-noise-control-anc = Noise cancelling
device-eq-preset-flat = Flat
device-eq-preset-bass-boost = Bass boost
device-eq-preset-focus = Focus
device-eq-preset-smiley = Smiley
device-eq-preset-custom = Custom
device-auto-off-never = Never
device-auto-off-1min = 1 minute
device-auto-off-5min = 5 minutes
device-auto-off-10min = 10 minutes
device-auto-off-15min = 15 minutes
device-auto-off-30min = 30 minutes
device-auto-off-60min = 60 minutes
device-sidetone-off = Off
device-sidetone-low = Low
device-sidetone-medium = Medium
device-sidetone-high = High

overview-title = Overview
overview-subtitle = Your devices at a glance.
overview-unlinked = { $hub } connected · { $class } off or out of range
overview-connected = Connected
# { $charge } is a charging state such as "Charging".
overview-connected-charge = Connected · { $charge }
overview-settings = Settings
overview-quick-controls = Quick controls
overview-eq-hint = Device preset.
overview-audiomix-hint = Sort apps into Game, Chat, Media and Aux and balance Game against Chat.
overview-manage = Manage
overview-balance = Game/Chat balance
overview-balance-off = Balances game and chat audio — with this slider or the hotkeys. Turn on AudioMix to use it.
overview-balance-off-dial = Balances game and chat audio — with this slider, the hotkeys or the dial on your device. Turn on AudioMix to use it.
# -- end of device section --

## AudioMix and presets (ui/audiomix.rs, ui/presets.rs, audio/)
# -- end of AudioMix section --

## Settings, onboarding, hotkeys, updates (ui/settings.rs, ui/onboarding.rs, hotkeys.rs, update.rs)
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
