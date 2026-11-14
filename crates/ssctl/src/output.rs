//! Human-readable printing of device state.

use ss_core::battery::{BatteryLevel, BatteryReport};
use ss_nova_pro::{Event, Status};

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
    println!("Auto-off:       {}", debug_or_dash(s.auto_off));
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
