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
mix-subtitle = Sortiere deine Apps in Game, Chat, Media und Aux und steuere jede Gruppe einzeln.
mix-switch-hint-dsp = Leitet den Ton der Apps durch OpenSeries. Wenn aus, greift OpenSeries überhaupt nicht in den Systemton ein — nutze das, wenn ein anderes Audio-Tool deine Apps übernehmen soll.
mix-switch-hint-volumes = Stellt die Lautstärke jeder App nach ihrem Kanal ein und gleicht Game und Chat aus. Wenn aus, ändert OpenSeries keine App-Lautstärken.
mix-balance-title = Game/Chat-Balance
mix-balance-hint = Folgt auch dem Drehregler an deinem Gerät und den Balance-Hotkeys.
mix-balance-split = Game { $game } %  ·  Chat { $chat } %
mix-balance-centre = Mitte
mix-output-title = Ausgabe
mix-play-through = Wiedergabe über
mix-play-through-hint = Meist dein Headset.
mix-backup-outputs = Ersatz-Ausgabegeräte
mix-backup-outputs-hint = Werden in dieser Reihenfolge genutzt, wenn die Ausgabe oben fehlt (auch als Ersatz für das Mikrofon).
mix-backup-when-headset-off = Ersatzgeräte nutzen, solange das Headset aus ist
mix-channels-title = Kanäle
mix-channels-windows-hint = Unter Windows stellen Kanäle die Lautstärke ihrer Apps ein. Equalizer, Boosts und Smart Volume pro Kanal gibt es unter macOS.
mix-all-channels = Alle Kanäle
mix-channel-sound = { $channel }-Klang
mix-smart-volume-hint = Gleicht laute und leise Passagen aus.
mix-smart-volume-off = Aus
mix-smart-volume-soft = Sanft
mix-smart-volume-balanced = Ausgewogen
mix-smart-volume-strong = Stark
mix-boost-bass = Bass
mix-boost-treble = Höhen
mix-boost-clarity = Sprachklarheit
mix-boost-volume = Lautstärke-Boost
mix-eq = Equalizer
mix-eq-flat = Neutral
mix-apps-title = Apps
mix-show-idle = Apps anzeigen, die gerade nichts abspielen
mix-show-idle-hint = Apps, die du nicht zugewiesen hast, laufen auf Media.
mix-no-apps = Gerade spielt keine App Ton ab. Starte etwas Audio, dann erscheint sie hier.
mix-app-playing = Spielt ab
mix-app-idle = Inaktiv
mix-app-not-running = Läuft nicht
mix-app-unassigned = nicht zugewiesen — läuft auf Media
mix-reset = Zurücksetzen
mix-reset-hint = Zurück zum automatischen Kanal: { $channel }
audio-source-own = deine Wahl
audio-source-catalogue = eingebaute Liste
audio-source-game-library = Spielebibliothek
audio-source-games-category = Kategorie Spiele
audio-paused = kein unterstütztes Gerät verbunden
audio-conflict = { $count ->
    [one] { $tools } läuft und nimmt ebenfalls App-Audio auf
   *[other] { $tools } laufen und nehmen ebenfalls App-Audio auf
}
audio-not-supported = auf diesem Betriebssystem nicht unterstützt
audio-output-missing = das gewählte Ausgabegerät ist nicht verbunden
audio-mic-missing = das gewählte Mikrofon ist nicht verbunden
audio-mic-is-virtual = der Systemeingang ist das OpenSeries Microphone selbst; wähle das Headset als Quelle
audio-mic-macos-only = Das virtuelle Mikrofon gibt es nur unter macOS (Windows bräuchte einen signierten Audiotreiber)
audio-unsupported-mix = AudioMix ist auf diesem Betriebssystem noch nicht verfügbar
audio-unsupported-mic = Das virtuelle Mikrofon ist auf diesem Betriebssystem noch nicht verfügbar
presets-label = Preset
presets-none = noch keine Presets
presets-delete = Löschen
presets-export = Exportieren…
presets-import = Importieren…
presets-name-hint = Name
presets-save = Aktuellen speichern
presets-applied = „{ $name }“ angewendet
presets-deleted = „{ $name }“ gelöscht
presets-imported-pick = „{ $name }“ importiert — wähle es zum Anwenden
presets-imported-as = „{ $name }“ als { $kind }-Preset importiert
presets-import-failed = Import fehlgeschlagen: { $error }
presets-replaced = „{ $name }“ ersetzt
presets-saved = „{ $name }“ gespeichert
presets-nothing-to-save = noch nichts zu speichern
presets-export-cancelled = Export abgebrochen
presets-exported = exportiert nach { $path }
presets-export-failed = Export fehlgeschlagen: { $error }
presets-file-filter = OpenSeries-Preset
presets-import-filter = Preset (OpenSeries- oder Sonar-JSON)
presets-device-eq-elsewhere = das ist ein Geräte-EQ-Preset; importiere es auf der Seite des Geräts
presets-kind-device-eq = Geräte-EQ
presets-kind-channel = Kanal
presets-kind-mic = Mikrofon
# -- end of AudioMix section --

## Settings, onboarding, hotkeys, updates (ui/settings.rs, ui/onboarding.rs, hotkeys.rs, update.rs)
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
