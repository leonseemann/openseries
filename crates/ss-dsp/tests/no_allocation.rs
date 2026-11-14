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
fn counter_detects_allocations() {
    // Control: proves the measurement is active, so a 0 above is meaningful.
    let info = allocation_counter::measure(|| {
        let v = std::hint::black_box(vec![0_u8; 64]);
        drop(v);
    });
    assert!(info.count_total >= 1);
}
