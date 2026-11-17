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
settings-title = Settings
settings-subtitle = How OpenSeries starts, notifies and reacts to keys.
settings-cancel = Cancel

settings-general = General
settings-autostart = Start at login
settings-autostart-hint = OpenSeries waits in the menu bar / tray.
settings-experimental = Experimental devices
settings-experimental-hint = Also use drivers that were not tested on hardware yet. They may not work as described.
settings-notifications = Notifications
settings-notifications-hint = Headset connected or disconnected, low battery, audio errors.
settings-notifications-test = Send test
settings-notifications-test-text = Test notification
settings-notifications-test-sent = Sent
settings-notifications-test-failed = Could not send: { $error }

settings-hotkeys = Hotkeys
settings-hotkeys-hint = Work everywhere, also while OpenSeries is in the background.
settings-hotkeys-clear = Clear
settings-hotkeys-change = Change
settings-hotkeys-recording = Press the new keys…
settings-hotkeys-not-set = Not set
settings-hotkeys-reset = Reset hotkeys to defaults
settings-hotkeys-taken = { $binding } is already used by "{ $action }".

settings-crackling = Crackling
settings-crackling-auto = Fix crackling automatically
settings-crackling-auto-hint = When the base station's USB audio stutters, OpenSeries reconnects it: about one second of silence instead of ongoing crackling.
settings-crackling-watching = Watching · fixed { $fixes ->
        [one] once
       *[other] { $fixes } times
    }
settings-crackling-watching-last = Watching · fixed { $fixes ->
        [one] once
       *[other] { $fixes } times
    }, last { $seconds } s ago

settings-updates = Updates
settings-updates-automatic = Check for updates automatically
settings-updates-automatic-hint = Looks for a new version on GitHub after start and every 6 hours.
settings-updates-status = Status
settings-updates-install = Install update
settings-updates-check-now = Check now
settings-updates-install-hint = Installing quits OpenSeries for a moment; it starts again by itself.
settings-updates-whats-new = What's new
settings-updates-not-checked = Not checked yet
settings-updates-checking = Checking…
settings-updates-up-to-date = Up to date ({ $version })
settings-updates-available = Version { $version } is available
settings-updates-downloading = Downloading { $version } … { $percent } %
settings-updates-installing = Installing { $version }…
settings-updates-check-failed = Check failed
settings-updates-unavailable = Not available

settings-about = About
settings-about-version = Version
settings-about-file = Settings file
settings-about-copy-path = Copy path
settings-about-copy-path-hint = Copies the path to the clipboard.
settings-about-getting-started = Getting started
settings-about-getting-started-hint = The checklist from the first start.
settings-about-show = Show
settings-about-quit = Quit OpenSeries
settings-about-quit-hint = Closing the window keeps OpenSeries running in the background.
settings-about-quit-button = Quit
settings-about-disclaimer = Independent open-source software (MIT OR Apache-2.0), free of charge. Not affiliated with SteelSeries.
settings-about-licences = Third-party licences
settings-reset = Reset settings
settings-reset-hint = Back to defaults; your presets are kept. The headset is not changed.
settings-reset-button = Reset…
settings-reset-confirm = Reset AudioMix, microphone, hotkeys, notifications and all other OpenSeries settings? AudioMix and the virtual microphone switch off and start at login is turned off.
settings-reset-everything = Reset everything

onboarding-welcome = Welcome to OpenSeries
onboarding-intro = A few steps to get everything working. You can open this again from Settings.
onboarding-done = Done
onboarding-turn-on = Turn on
onboarding-install-driver = Install driver…
onboarding-driver-installed = Driver installed
onboarding-driver-failed = Driver: { $error }
onboarding-device = Connect your device
onboarding-device-hint = Plug it into this computer. The Arctis Nova Pro Wireless connects with its base station by USB (on macOS the USB-1 port).
onboarding-headset = Turn the headset on
onboarding-headset-hint = Switch the headset on; it links to the base station by itself.
onboarding-audiomix = Turn on AudioMix
onboarding-audiomix-hint-macos = Routes app audio through Game / Chat / Media / Aux with EQ and ChatMix. macOS asks for System Audio Recording permission (Privacy & Security).
onboarding-audiomix-hint = Sets each app's volume by its channel (Game / Chat / Media / Aux) and ChatMix. Per-channel EQ is available on macOS only.
onboarding-mic-driver = Install the microphone driver
onboarding-mic-driver-hint = Adds "OpenSeries Microphone" to macOS. Needs your administrator password; audio pauses for about a second.
onboarding-virtual-mic = Turn on the virtual microphone
onboarding-virtual-mic-hint = Feeds your processed voice into "OpenSeries Microphone" — select it in Discord, Zoom or OBS.
onboarding-autostart = Start OpenSeries at login (optional)
onboarding-autostart-hint = Keeps the headset settings and AudioMix available after login.

hotkey-chatmix-game = Game/Chat balance towards Game
hotkey-chatmix-chat = Game/Chat balance towards Chat
hotkey-mic-mute = Mute virtual microphone
hotkey-reconnect-usb = Reconnect USB (fixes crackling)
hotkey-invalid = "{ $binding }" is not a valid key combination
hotkey-taken-elsewhere = { $binding } is already used by another app (or a second OpenSeries). Choose another combination in Settings → Hotkeys.
hotkey-register-failed = { $binding } could not be registered ({ $error }).
hotkey-needs-modifier = Use at least Ctrl or Alt (Cmd on macOS) with the key.
hotkey-key-unusable = { $key } cannot be used as a hotkey.

update-notification = OpenSeries { $version } is available. Install it from the tray menu or Settings → Updates.
update-error-folder = creating { $path }: { $error }
update-error-signature = damaged signature file
update-error-save = saving the update: { $error }
update-error-interrupted = download interrupted: { $error }
update-error-download = download: { $error }
update-error-no-release = no release has been published yet
update-error-rate-limited = GitHub limits update checks right now; try again later
update-error-http = GitHub answered with error { $code }
update-error-unreachable = could not reach GitHub ({ $error })
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
