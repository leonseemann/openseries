//! Real-time safety: processing must not allocate (`docs/architecture/dsp.md` §3,
//! `mic-chain.md` §3). `allocation-counter` installs a counting global allocator for this binary.

use ss_dsp::{
    DEFAULT_AUTO_GATE_OFFSET_DB, DEFAULT_DENOISE_MIX, GateMode, MicChain, MicChainSettings,
    Processor, voice_stabilizer,
};

#[test]
fn full_mic_chain_does_not_allocate_while_processing() {
    let mut chain = MicChain::new(48_000.0).unwrap();
    let mut eq = ss_core::eq::ParametricEqSettings::flat();
    eq.enabled = true;
    chain.set(&MicChainSettings {
        denoise: Some(DEFAULT_DENOISE_MIX),
        gate: Some(GateMode::Automatic {
            offset_db: DEFAULT_AUTO_GATE_OFFSET_DB,
        }),
        stabilizer: Some(voice_stabilizer()),
        eq,
        gain_db: 3.0,
    });
    let mut block = vec![0.0_f32; 512];
    let mut phase = 0.0_f32;
    let mut fill = |block: &mut [f32]| {
        for sample in block {
            phase += 0.05;
            *sample = 0.2 * phase.sin();
        }
    };
    // Warm-up: lets lazily initialised internals (e.g. FFT plans) settle.
    for _ in 0..10 {
        fill(&mut block);
        chain.process(&mut block);
    }

    let info = allocation_counter::measure(|| {
        for _ in 0..200 {
            fill(&mut block);
            chain.process(&mut block);
        }
    });
    assert_eq!(info.count_total, 0, "allocations on the audio path");
}

#[test]
fn changing_mic_chain_settings_does_not_allocate() {
    // Live changes are applied on the audio thread (`mic-chain.md` §3), so `set` must be
    // real-time safe for every block being switched on, off and retuned.
    let mut chain = MicChain::new(48_000.0).unwrap();
    let mut eq = ss_core::eq::ParametricEqSettings::flat();
    chain.warm_up(); // as the mic feed does in its first callback
    let mut block = vec![0.1_f32; 512];
    for _ in 0..10 {
        chain.process(&mut block);
    }
    let variants: Vec<MicChainSettings> = (0..8_u8)
        .map(|i| {
            eq.enabled = i % 2 == 0;
            MicChainSettings {
                denoise: (i % 3 != 0).then_some(f32::from(i) / 10.0),
                gate: match i % 3 {
                    0 => None,
                    1 => Some(GateMode::Manual {
                        threshold_db: -50.0 - f32::from(i),
                    }),
                    _ => Some(GateMode::Automatic {
                        offset_db: DEFAULT_AUTO_GATE_OFFSET_DB,
                    }),
                },
                stabilizer: (i % 2 == 1).then(voice_stabilizer),
                eq,
                gain_db: f32::from(i) - 4.0,
            }
        })
        .collect();
    let info = allocation_counter::measure(|| {
        for settings in &variants {
            chain.set(settings);
            chain.process(&mut block);
        }
    });
    assert_eq!(info.count_total, 0, "allocations while changing settings");
}

#[test]
fn counter_detects_allocations() {
    // Control: proves the measurement is active, so a 0 above is meaningful.
    let info = allocation_counter::measure(|| {
        let v = std::hint::black_box(vec![0_u8; 64]);
        drop(v);
    });
    assert!(info.count_total >= 1);
}
