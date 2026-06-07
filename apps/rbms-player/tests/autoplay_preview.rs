//! Headless verification of the song-select autoplay-preview pipeline — the path taken when a chart
//! defines no `#PREVIEW` file: parse -> model -> autoplay keysound schedule -> keysound decode. The
//! only stage this cannot exercise is the cpal device output, which is shared with gameplay audio.

use std::path::Path;

#[test]
fn autoplay_preview_extracts_events_and_decodable_keysounds() {
    let bms = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/preview-demo/autoplay-demo.bms");
    let bytes = std::fs::read(&bms).expect("autoplay-demo fixture is committed");
    let src = rbms_parser::parse_with(&bytes, Default::default());
    assert!(src.headers.preview.trim().is_empty(), "fixture must define NO #PREVIEW so it exercises the autoplay path");

    let name = bms.to_string_lossy();
    let mode = rbms_chart::detect_mode(&src, &name);
    let model = rbms_chart::to_model(&src, mode);

    let mut sched: Vec<(i64, u32)> = Vec::new();
    let mut player = rbms_play::Player::new(model.clone(), true);
    let end = player.last_time_us() + 1_000_000;
    player.update(end, |ev| {
        if ev.wav >= 0 {
            sched.push((ev.at_us, ev.wav as u32));
        }
    });
    sched.sort_by_key(|s| s.0);
    assert!(!sched.is_empty(), "autoplay preview must emit keysound events for a chart with notes");

    let dir = bms.parent().unwrap();
    let mut decoded = 0usize;
    for name in model.wavmap.iter().filter(|n| !n.trim().is_empty()) {
        let path = dir.join(name);
        let data = std::fs::read(&path).unwrap_or_else(|_| panic!("keysound {} present", path.display()));
        let dec = rbms_audio::decode_bytes(data, path.extension().and_then(|x| x.to_str())).expect("keysound decodes");
        assert!(dec.samples.iter().any(|&s| s != 0.0), "keysound must decode to audible PCM");
        decoded += 1;
    }
    assert!(decoded >= 1, "the fixture references at least one keysound");
}
