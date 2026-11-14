//! Real-time safety of the drift buffer: `write` and `read` run on audio threads and must not
//! allocate (`docs/architecture/windows-virtual-driver.md` §2.4). `allocation-counter` installs a
//! counting global allocator for this test binary.

use ss_engine::drift_buffer;

#[test]
fn drift_buffer_does_not_allocate_on_the_audio_path() {
    let (mut writer, mut reader, _stats) = drift_buffer(2, 4800, 960);
    let input = vec![0.25_f32; 2 * 481];
    let mut output = vec![0.0_f32; 2 * 480];
    // Prime outside the measurement (priming itself does not allocate either, but keep the
    // measured part to the steady state the callbacks see).
    writer.write(&input);
    writer.write(&input);
    reader.read(&mut output);

    let info = allocation_counter::measure(|| {
        for _ in 0..1000 {
            writer.write(&input);
            reader.read(&mut output);
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
