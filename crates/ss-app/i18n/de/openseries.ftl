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
# -- end of AudioMix section --

## Settings, onboarding, hotkeys, updates (ui/settings.rs, ui/onboarding.rs, hotkeys.rs, update.rs)
# -- end of settings section --

## Microphone, window frame, shared widgets, notifications (ui/mic.rs, ui/mod.rs, ui/components.rs, ui/widgets.rs, alerts.rs, platform/)
# -- end of microphone section --
