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
