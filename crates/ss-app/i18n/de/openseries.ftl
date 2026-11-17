# OpenSeries — Deutsch.

language-name = Deutsch

## Tray menu

tray-audiomix = AudioMix
tray-virtual-mic = Virtuelles Mikrofon
tray-reconnect-usb = USB neu verbinden (behebt Knistern)
tray-check-updates = Nach Updates suchen
tray-open = OpenSeries öffnen…
tray-quit = OpenSeries beenden
tray-paused = Pausiert · kein unterstütztes Gerät verbunden
tray-no-device = Kein Gerät verbunden
tray-device-off = { $device } · aus
tray-device-connected = { $device } verbunden
tray-device-battery = { $device } · ≈{ $percent } %
tray-checking-updates = Suche nach Updates…
tray-downloading-update = OpenSeries { $version } wird heruntergeladen…
tray-installing-update = OpenSeries { $version } wird installiert…
tray-install-update = Update { $version } installieren…

## Settings → General → Language

settings-language = Sprache
settings-language-hint = Weitere Sprachen als i18n/<Code>/openseries.ftl im Konfigurationsordner hinzufügen.
settings-language-system = Systemstandard

## Device pages and overview (ui/device.rs, ui/overview.rs)
# -- end of device section --

## AudioMix and presets (ui/audiomix.rs, ui/presets.rs, audio/)
# -- end of AudioMix section --

## Settings, onboarding, hotkeys, updates (ui/settings.rs, ui/onboarding.rs, hotkeys.rs, update.rs)
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# Microphone page
mic-title = Mikrofon
mic-subtitle = Eine klarere Stimme für Discord, Zoom und OBS: Rauschunterdrückung, Noise Gate, Stabilisator und EQ.
mic-unavailable-title = Auf diesem Computer nicht verfügbar
mic-unavailable-body = Das bearbeitete „OpenSeries Microphone“ braucht ein virtuelles Audiogerät. Unter macOS ist es verfügbar. Unter Windows wäre dafür ein kostenpflichtiger, signierter Audiotreiber nötig, und OpenSeries bleibt kostenlos – deshalb ist diese Seite hier leer.
mic-unavailable-headset-hint = Mikrofonlautstärke, Mithörton und die Stummschalt-Leuchte findest du auf der Headset-Seite.
mic-virtual = Virtuelles Mikrofon
mic-virtual-hint = Wähle in deinen Apps „OpenSeries Microphone“ als Eingang.
mic-install-driver-hint = Installiere den Mikrofontreiber einmalig: Einstellungen → Erste Schritte anzeigen → Treiber installieren.
mic-source = Quelle
mic-input = Mikrofon
mic-input-hint = Meist dein Headset.
mic-as-default = Als Systemeingang verwenden
mic-as-default-hint = „OpenSeries Microphone“ wird zum Standardeingang, solange es läuft.
mic-default-input-error = Standardeingang: { $error }
mic-backup = Ersatzmikrofone
mic-processing = Bearbeitung
mic-denoise = Rauschunterdrückung
mic-denoise-hint = Entfernt Lüfter, Tastaturgeräusche und Hintergrundrauschen.
mic-stabilizer = Lautstärke-Stabilisator
mic-stabilizer-hint = Gleicht leises und lautes Sprechen aus.
mic-gain = Ausgangsverstärkung
mic-gain-hint = Insgesamt lauter oder leiser.
mic-presets = Presets
mic-gate = Noise Gate
mic-gate-hint = Stille zwischen Sätzen. Automatisch passt sich an deinen Raum an.
mic-gate-off = Aus
mic-gate-automatic = Automatisch
mic-gate-manual = Manuell
mic-gate-threshold = Schwelle
mic-gate-threshold-hint = Leisere Geräusche werden stummgeschaltet.
mic-gate-sensitivity = Empfindlichkeit
mic-gate-sensitivity-hint = Wie weit deine Stimme über dem Raumgeräusch liegen muss.

# Window frame: sidebar and banners above every page
nav-overview = Übersicht
nav-device = Gerät
nav-audiomix = AudioMix
nav-mic = Mikrofon
nav-settings = Einstellungen
nav-devices-heading = GERÄTE
nav-no-devices = Keine verbunden
nav-status-mic = Mikrofon · { $state }
nav-status-audiomix = AudioMix · { $state }
nav-no-device-connected = Kein Gerät verbunden
nav-device-reading = { $device } · wird gelesen…
nav-device-off = { $device } · aus
nav-device-battery = { $device } · { $percent } %
nav-device-connected = { $device } · verbunden
nav-paused-banner = OpenSeries ist pausiert: Kein unterstütztes Gerät ist verbunden. Bis eines zurück ist, lässt OpenSeries App-Audio, Mikrofone und Tastenkürzel unberührt, damit andere Geräte ungestört funktionieren.
nav-update-available = OpenSeries { $version } ist verfügbar.
nav-install-update = Update installieren
nav-update-details = Details
nav-hotkeys-unavailable = Tastenkürzel nicht verfügbar: { $error }

# Shared widgets
common-off = Aus
common-on = An
common-paused = Pausiert
common-error = Fehler
common-paused-reason = Pausiert: { $reason }
common-system-default = Systemstandard
common-not-connected = { $device } (nicht verbunden)
common-add-backup = Ersatz hinzufügen…

# Desktop notifications
alert-linked = { $device } verbunden
alert-unlinked = { $device } getrennt (aus oder außer Reichweite)
alert-gone = { $device }: { $hub } getrennt – ausgesteckt oder auf einen anderen Computer umgeschaltet
alert-back = { $device }: { $hub } verbunden
alert-battery-low = { $device }: Akku schwach (≈{ $percent } %)
alert-service-failed = { $service } gestoppt: { $message }
alert-service-audiomix = AudioMix
alert-service-virtual-mic = Virtuelles Mikrofon

# Messages from the operating system adapters
platform-autostart-failed = Bei Anmeldung starten: { $error }
platform-autostart-unavailable = Starten bei der Anmeldung ist auf diesem Betriebssystem nicht verfügbar.
platform-usb-reset-unavailable = USB neu verbinden ist auf diesem Betriebssystem noch nicht verfügbar.
platform-crackling-watch-unavailable = Die Knister-Überwachung ist auf diesem Betriebssystem noch nicht verfügbar.
platform-mic-driver-unavailable = Für dieses Betriebssystem gibt es noch keinen Treiber für das virtuelle Mikrofon.
platform-update-unavailable = Für dieses Betriebssystem gibt es keine Update-Pakete.
platform-driver-install-cancelled = Installation abgebrochen.
# -- end of microphone section --
