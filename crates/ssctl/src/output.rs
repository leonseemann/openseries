//! Human-readable printing of device state.

use ss_core::battery::{BatteryLevel, BatteryReport};
use ss_nova_pro::{Display, Event, Settings, Status};

fn percent(level: Option<BatteryLevel>) -> String {
    level.map_or_else(|| "–".to_owned(), |l| format!("{} %", l.percent()))
}

/// Prints the status block.
pub fn print_status(s: &Status) {
    println!("Connected:      {}", if s.connected { "yes" } else { "no" });
    println!("Headset battery {}", percent(s.headset_battery));
    println!("Spare battery   {}", percent(s.spare_battery));
    println!("Charging:       {:?}", s.charge_state);
    println!("Noise control:  {}", debug_or_dash(s.noise_control));
    println!(
        "Transparency:   {}",
        s.transparency
            .map_or_else(|| "–".to_owned(), |t| t.get().to_string())
    );
    println!("Mic muted:      {}", if s.mic_muted { "yes" } else { "no" });
    println!(
        "Mic LED:        {}",
        level_or_dash(s.mic_led.map(ss_core::headset::MicLedBrightness::get))
    );
    println!("Auto-off:       {}", debug_or_dash(s.auto_off));
}

/// Prints the audio settings block.
pub fn print_settings(s: &Settings) {
    println!("EQ preset:      {}", debug_or_dash(s.eq_preset));
    let curve = s.custom_eq.map_or_else(
        || "–".to_owned(),
        |eq| {
            let gains: Vec<String> = eq.bands.iter().map(|g| format!("{:+}", g.db())).collect();
            gains.join(" ")
        },
    );
    println!("Custom EQ (dB): {curve}");
    println!(
        "Mic volume:     {}",
        level_or_dash(s.mic_volume.map(ss_core::headset::MicVolume::get))
    );
    println!("Sidetone:       {}", debug_or_dash(s.sidetone));
    let mix = s.chatmix.map_or_else(
        || "–".to_owned(),
        |m| format!("game {} % / chat {} %", m.game.get(), m.chat.get()),
    );
    println!("ChatMix:        {mix}");
}

/// Prints the display settings block.
pub fn print_display(d: Display) {
    println!(
        "OLED brightness {}",
        level_or_dash(d.brightness.map(ss_core::headset::DisplayBrightness::get))
    );
    println!(
        "Mixer icon:     {}",
        if d.mixer_present { "on" } else { "off" }
    );
}

fn level_or_dash(level: Option<u8>) -> String {
    level.map_or_else(|| "–".to_owned(), |l| l.to_string())
}

/// Prints a battery report.
pub fn print_battery(b: BatteryReport) {
    println!(
        "Headset battery {} ({:?})",
        percent(Some(b.headset)),
        b.state
    );
    println!("Spare battery   {}", percent(b.spare));
}

/// Prints one event on a single line.
pub fn print_event(event: &Event) {
    match event {
        Event::ChatMix(m) => println!(
            "ChatMix   game {:>3} %  chat {:>3} %",
            m.game.get(),
            m.chat.get()
        ),
        Event::Battery(b) => println!(
            "Battery   headset {}  spare {}  {:?}",
            percent(Some(b.headset)),
            percent(b.spare),
            b.state
        ),
        Event::Unrecognized { report_id, opcode } => {
            println!("Unknown   report {report_id:#04x} opcode {opcode:#04x}");
        }
        other => println!("{other:?}"),
    }
}

fn debug_or_dash<T: std::fmt::Debug>(value: Option<T>) -> String {
    value.map_or_else(|| "–".to_owned(), |v| format!("{v:?}"))
}

/// Prints a report as offset-annotated hex (16 bytes per line).
pub fn print_hex(report: &[u8]) {
    for (row, chunk) in report.chunks(16).enumerate() {
        let bytes: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        println!("[{:>2}] {}", row * 16, bytes.join(" "));
    }
}
