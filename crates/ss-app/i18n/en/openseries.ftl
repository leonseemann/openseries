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
