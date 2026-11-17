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
device-title = Gerät
device-gone = Das Gerät ist nicht mehr verbunden.
device-subtitle = { $class } – Einstellungen werden auf dem Gerät gespeichert.
device-experimental = Experimentelle Unterstützung: Dieses Gerät wurde vom OpenSeries-Team nicht mit echter Hardware getestet. Einstellungen funktionieren eventuell nicht wie beschrieben.
device-reading = { $hub } wird ausgelesen…
device-unlinked = { $hub } ist verbunden, aber { $class } ist ausgeschaltet oder außer Reichweite. Einstellungen, die in { $hub } gespeichert sind, lassen sich weiterhin ändern.
device-none = Kein unterstütztes Gerät verbunden
device-none-hint = Verbinde dein Gerät mit diesem Computer. Das Arctis Nova Pro Wireless wird über seine Basisstation verbunden (unter macOS am Anschluss USB-1). Ist sie eingesteckt, ist sie vielleicht auf den anderen USB-Eingang umgeschaltet – schalte ihn an der Basisstation zurück.

device-group-noise-control = Geräuschsteuerung
device-group-microphone = Mikrofon
device-group-equalizer = Equalizer
device-group-power = Energie und Display
device-group-device = Gerät
device-group-more = Weitere

device-noise-control-mode = Modus
device-noise-control-mode-hint = Auch mit der Taste am Gerät umschaltbar.
device-transparency-strength = Stärke des Transparenzmodus
device-transparency-strength-hint = Wie viel du von deiner Umgebung hörst.
device-mic-mute = Status
device-mic-mute-hint = Mit der Taste am Gerät stummgeschaltet.
device-mic-volume = Mikrofonlautstärke
device-sidetone = Mithörton
device-sidetone-hint = Hör deine eigene Stimme, während du sprichst.
device-mic-led-brightness = Helligkeit der Stummschalt-LED
device-mic-led-brightness-hint = Die Leuchte am Mikrofon, während es stummgeschaltet ist.
device-eq-preset = Preset
device-eq-preset-hint = Auf dem Gerät gespeichert; wirkt auf alles, was es abspielt.
device-custom-curve = Eigene Kurve
device-auto-off = Bei Inaktivität ausschalten
device-auto-off-hint = Schaltet nach dieser Zeit ohne Ton ab.
device-display-brightness = Display-Helligkeit
device-firmware = Firmware
device-usb-replug = USB neu verbinden
device-usb-replug-hint = Behebt Knistern: wie das Ab- und Wiederanstecken des Kabels (der Ton setzt kurz aus).
device-setting = Einstellung

device-battery = Akku
device-spare-battery = Ersatzakku
device-spare-battery-hint = Der Akku, der in der Basisstation lädt.
device-charge-discharging = Akkubetrieb
device-charge-charging = Wird geladen
device-charge-plugged = Angeschlossen
device-charge-unknown = Akkustatus unbekannt

device-unknown = Unbekannt
device-versions-unknown = unbekannt
device-versions-hint = Versionen der Komponenten, wie das Gerät sie meldet. Zu welchem Teil welche Nummer gehört, ist eventuell nicht bekannt.
device-reconnect = Neu verbinden
device-reconnect-unsupported = Auf diesem Betriebssystem noch nicht verfügbar.
device-muted = Stumm
device-live = Aktiv
device-on = An
device-off = Aus
device-connection = Verbindung
device-release = Release { $version }
device-no-curve = Das Gerät hat keine eigene Kurve gemeldet.
device-curve-range = { $min } … { $max } dB pro Band, von tief nach hoch
device-reset-flat = Auf linear zurücksetzen
device-band-low = Tief
device-band-high = Hoch

device-class-headset = Headset
device-class-keyboard = Tastatur
device-class-mouse = Maus
device-class-other = Gerät
device-class-headset-inline = Headset
device-class-keyboard-inline = Tastatur
device-class-mouse-inline = Maus
device-class-other-inline = Gerät

device-hub-base-station = Basisstation
device-noise-control-off = Aus
device-noise-control-transparency = Transparenzmodus
device-noise-control-anc = Geräuschunterdrückung
device-eq-preset-flat = Linear
device-eq-preset-bass-boost = Bass-Boost
device-eq-preset-focus = Fokus
device-eq-preset-smiley = Smiley
device-eq-preset-custom = Eigene
device-auto-off-never = Nie
device-auto-off-1min = 1 Minute
device-auto-off-5min = 5 Minuten
device-auto-off-10min = 10 Minuten
device-auto-off-15min = 15 Minuten
device-auto-off-30min = 30 Minuten
device-auto-off-60min = 60 Minuten
device-sidetone-off = Aus
device-sidetone-low = Niedrig
device-sidetone-medium = Mittel
device-sidetone-high = Hoch

overview-title = Übersicht
overview-subtitle = Deine Geräte auf einen Blick.
overview-unlinked = { $hub } verbunden · { $class } aus oder außer Reichweite
overview-connected = Verbunden
overview-connected-charge = Verbunden · { $charge }
overview-settings = Einstellungen
overview-quick-controls = Schnellzugriff
overview-eq-hint = Preset des Geräts.
overview-audiomix-hint = Ordne Apps den Kanälen Game, Chat, Media und Aux zu und gleiche Game gegen Chat ab.
overview-manage = Verwalten
overview-balance = Game/Chat-Balance
overview-balance-off = Regelt das Verhältnis von Spiel- und Chat-Ton – mit diesem Regler oder den Tastenkürzeln. Schalte AudioMix ein, um sie zu nutzen.
overview-balance-off-dial = Regelt das Verhältnis von Spiel- und Chat-Ton – mit diesem Regler, den Tastenkürzeln oder dem Drehregler an deinem Gerät. Schalte AudioMix ein, um sie zu nutzen.
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
