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
settings-title = Einstellungen
settings-subtitle = Wie OpenSeries startet, benachrichtigt und auf Tasten reagiert.
settings-cancel = Abbrechen

settings-general = Allgemein
settings-autostart = Bei der Anmeldung starten
settings-autostart-hint = OpenSeries wartet in der Menüleiste / im Infobereich.
settings-experimental = Experimentelle Geräte
settings-experimental-hint = Auch Treiber verwenden, die noch nicht mit echter Hardware getestet wurden. Sie funktionieren eventuell nicht wie beschrieben.
settings-notifications = Mitteilungen
settings-notifications-hint = Headset verbunden oder getrennt, Akku schwach, Audiofehler.
settings-notifications-test = Test senden
settings-notifications-test-text = Testmitteilung
settings-notifications-test-sent = Gesendet
settings-notifications-test-failed = Senden fehlgeschlagen: { $error }

settings-hotkeys = Tastenkürzel
settings-hotkeys-hint = Funktionieren überall, auch wenn OpenSeries im Hintergrund läuft.
settings-hotkeys-clear = Entfernen
settings-hotkeys-change = Ändern
settings-hotkeys-recording = Neue Tasten drücken…
settings-hotkeys-not-set = Nicht belegt
settings-hotkeys-reset = Tastenkürzel zurücksetzen
settings-hotkeys-taken = { $binding } ist bereits für „{ $action }“ belegt.

settings-crackling = Knistern
settings-crackling-auto = Knistern automatisch beheben
settings-crackling-auto-hint = Wenn das USB-Audio der Basisstation stockt, verbindet OpenSeries sie neu: etwa eine Sekunde Stille statt anhaltendem Knistern.
settings-crackling-watching = Aktiv · { $fixes ->
        [one] einmal behoben
       *[other] { $fixes }-mal behoben
    }
settings-crackling-watching-last = Aktiv · { $fixes ->
        [one] einmal behoben
       *[other] { $fixes }-mal behoben
    }, zuletzt vor { $seconds } s

settings-updates = Updates
settings-updates-automatic = Automatisch nach Updates suchen
settings-updates-automatic-hint = Sucht nach dem Start und alle 6 Stunden auf GitHub nach einer neuen Version.
settings-updates-status = Status
settings-updates-install = Update installieren
settings-updates-check-now = Jetzt suchen
settings-updates-install-hint = Beim Installieren wird OpenSeries kurz beendet und startet danach von selbst neu.
settings-updates-whats-new = Neuerungen
settings-updates-not-checked = Noch nicht gesucht
settings-updates-checking = Suche läuft…
settings-updates-up-to-date = Aktuell ({ $version })
settings-updates-available = Version { $version } ist verfügbar
settings-updates-downloading = { $version } wird heruntergeladen … { $percent } %
settings-updates-installing = { $version } wird installiert…
settings-updates-check-failed = Suche fehlgeschlagen
settings-updates-unavailable = Nicht verfügbar

settings-about = Über
settings-about-version = Version
settings-about-file = Einstellungsdatei
settings-about-copy-path = Pfad kopieren
settings-about-copy-path-hint = Kopiert den Pfad in die Zwischenablage.
settings-about-getting-started = Erste Schritte
settings-about-getting-started-hint = Die Checkliste vom ersten Start.
settings-about-show = Anzeigen
settings-about-quit = OpenSeries beenden
settings-about-quit-hint = Wird das Fenster geschlossen, läuft OpenSeries im Hintergrund weiter.
settings-about-quit-button = Beenden
settings-about-disclaimer = Unabhängige Open-Source-Software (MIT OR Apache-2.0), kostenlos. Nicht mit SteelSeries verbunden.
settings-about-licences = Lizenzen von Drittanbietern
settings-reset = Einstellungen zurücksetzen
settings-reset-hint = Zurück auf die Standardwerte; deine Presets bleiben erhalten. Am Headset ändert sich nichts.
settings-reset-button = Zurücksetzen…
settings-reset-confirm = AudioMix, Mikrofon, Tastenkürzel, Mitteilungen und alle anderen OpenSeries-Einstellungen zurücksetzen? AudioMix und das virtuelle Mikrofon werden ausgeschaltet, ebenso der Start bei der Anmeldung.
settings-reset-everything = Alles zurücksetzen

onboarding-welcome = Willkommen bei OpenSeries
onboarding-intro = Ein paar Schritte, damit alles funktioniert. Du kannst das jederzeit in den Einstellungen wieder öffnen.
onboarding-done = Fertig
onboarding-turn-on = Einschalten
onboarding-install-driver = Treiber installieren…
onboarding-driver-installed = Treiber installiert
onboarding-driver-failed = Treiber: { $error }
onboarding-device = Gerät anschließen
onboarding-device-hint = Schließe es an diesen Computer an. Das Arctis Nova Pro Wireless wird über seine Basisstation per USB verbunden (unter macOS am Anschluss USB-1).
onboarding-headset = Headset einschalten
onboarding-headset-hint = Schalte das Headset ein; es verbindet sich von selbst mit der Basisstation.
onboarding-audiomix = AudioMix einschalten
onboarding-audiomix-hint-macos = Leitet den Ton von Apps über Game / Chat / Media / Aux mit EQ und ChatMix. macOS fragt nach der Berechtigung für Systemaudioaufnahme (Datenschutz & Sicherheit).
onboarding-audiomix-hint = Regelt die Lautstärke jeder App über ihren Kanal (Game / Chat / Media / Aux) und ChatMix. EQ pro Kanal gibt es nur unter macOS.
onboarding-mic-driver = Mikrofontreiber installieren
onboarding-mic-driver-hint = Fügt „OpenSeries Microphone“ zu macOS hinzu. Braucht dein Administratorpasswort; der Ton setzt etwa eine Sekunde aus.
onboarding-virtual-mic = Virtuelles Mikrofon einschalten
onboarding-virtual-mic-hint = Leitet deine bearbeitete Stimme in „OpenSeries Microphone“ – wähle es in Discord, Zoom oder OBS aus.
onboarding-autostart = OpenSeries bei der Anmeldung starten (optional)
onboarding-autostart-hint = Hält die Headset-Einstellungen und AudioMix nach der Anmeldung bereit.

hotkey-chatmix-game = Game/Chat-Balance Richtung Game
hotkey-chatmix-chat = Game/Chat-Balance Richtung Chat
hotkey-mic-mute = Virtuelles Mikrofon stummschalten
hotkey-reconnect-usb = USB neu verbinden (behebt Knistern)
hotkey-invalid = „{ $binding }“ ist keine gültige Tastenkombination
hotkey-taken-elsewhere = { $binding } wird bereits von einer anderen App (oder einem zweiten OpenSeries) verwendet. Wähle unter Einstellungen → Tastenkürzel eine andere Kombination.
hotkey-register-failed = { $binding } konnte nicht registriert werden ({ $error }).
hotkey-needs-modifier = Verwende mindestens Strg oder Alt (unter macOS Cmd) zusammen mit der Taste.
hotkey-key-unusable = { $key } kann nicht als Tastenkürzel verwendet werden.

update-notification = OpenSeries { $version } ist verfügbar. Installiere es über das Menü in der Menüleiste oder unter Einstellungen → Updates.
update-error-folder = { $path } konnte nicht angelegt werden: { $error }
update-error-signature = Signaturdatei beschädigt
update-error-save = Update konnte nicht gespeichert werden: { $error }
update-error-interrupted = Download unterbrochen: { $error }
update-error-download = Download: { $error }
update-error-no-release = Es wurde noch keine Version veröffentlicht
update-error-rate-limited = GitHub begrenzt Update-Abfragen gerade; versuche es später noch einmal
update-error-http = GitHub hat mit Fehler { $code } geantwortet
update-error-unreachable = GitHub ist nicht erreichbar ({ $error })
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
