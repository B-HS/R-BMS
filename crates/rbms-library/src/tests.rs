use crate::*;

const US_PER_SECOND: i64 = 1_000_000;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/preview-demo")
}

#[test]
fn is_chart_accepts_every_chart_extension_in_any_case() {
    for name in ["a.bms", "a.BME", "a.Bml", "a.pms", "a.bmson", "a.BMSON", "deep/dir/song.bms"] {
        assert!(is_chart(Path::new(name)), "{name} is a chart");
    }
}

#[test]
fn is_chart_rejects_non_chart_files() {
    for name in ["a.wav", "a.ogg", "a.png", "a.bms.bak", "a", "dir/"] {
        assert!(!is_chart(Path::new(name)), "{name} is not a chart");
    }
}

#[test]
fn compute_chart_detail_derives_notes_length_and_bpm_range() {
    let path = samples_dir().join("autoplay-demo.bms");
    let d = compute_chart_detail(&path, Mode::BEAT_7K).expect("the committed sample chart parses");
    assert!(d.notes > 0, "the sample chart has playable notes");
    assert!(d.duration_us > 0, "the sample chart has a length");
    assert!(d.bpm_min > 0.0 && d.bpm_max >= d.bpm_min, "the BPM range is populated and ordered");
    assert!(!d.density.is_empty(), "the density histogram is populated");
}

#[test]
fn compute_chart_detail_measures_the_radar_of_a_mode_that_has_one() {
    let path = samples_dir().join("autoplay-demo.bms");
    let d = compute_chart_detail(&path, Mode::BEAT_7K).expect("the committed sample chart parses");
    let radar = d.radar.expect("the detail was measured");
    assert!(radar.is_current());
    assert!(radar.raw.is_some_and(|raw| raw.notes > 0.0), "the sample chart has press events per second: {radar:?}");
}

#[test]
fn compute_chart_detail_of_a_nine_key_chart_says_it_has_no_radar() {
    let path = samples_dir().join("autoplay-demo.bms");
    let d = compute_chart_detail(&path, Mode::POPN_9K).expect("the committed sample chart parses");
    assert_eq!(d.radar, Some(RadarRow { version: rbms_chart::RADAR_VERSION, raw: None }));
}

#[test]
fn compute_chart_detail_of_a_chart_years_long_has_no_density_histogram() {
    let path = std::env::temp_dir().join(format!("rbms_library_years_long_{}.bms", std::process::id()));
    std::fs::write(&path, "#BPM 0.0001\n#00111:01\n#99911:01\n").expect("write the fixture chart");
    let detail = compute_chart_detail(&path, Mode::BEAT_7K);
    let _ = std::fs::remove_file(&path);
    let detail = detail.expect("the chart parses");
    assert_eq!(detail.notes, 2);
    assert!(
        detail.duration_us / US_PER_SECOND > rbms_chart::MAX_TABLED_SECONDS as i64,
        "the fixture runs {} us, which is not past the bound",
        detail.duration_us
    );
    assert!(detail.density.is_empty());
    assert_eq!((detail.peak_density, detail.avg_density, detail.end_density), (0.0, 0.0, 0.0));
}

#[test]
fn compute_chart_detail_of_a_missing_file_is_none() {
    let missing = std::env::temp_dir().join(format!("rbms_library_no_chart_{}.bms", std::process::id()));
    let _ = std::fs::remove_file(&missing);
    assert!(compute_chart_detail(&missing, Mode::BEAT_7K).is_none(), "an unreadable chart yields no detail");
}

fn entry(md5: &str, title: &str) -> SongEntry {
    SongEntry {
        path: PathBuf::from(format!("/songs/{title}.bms")),
        title: title.into(),
        subtitle: String::new(),
        artist: String::new(),
        genre: String::new(),
        maker: String::new(),
        level: "5".into(),
        difficulty: 2,
        init_bpm: 150.0,
        rank: 2,
        total: 300.0,
        mode: Mode::BEAT_7K,
        md5: md5.into(),
        stagefile: String::new(),
        banner: String::new(),
        preview: String::new(),
    }
}

#[test]
fn library_preserves_entry_order() {
    let lib = Library::from_songs(vec![entry("AA", "one"), entry("BB", "two")]);
    assert_eq!(lib.len(), 2);
    assert!(!lib.is_empty());
    assert_eq!(lib.songs()[0].title, "one", "indices into songs() stay the scan order");
    assert_eq!(lib.md5s().collect::<Vec<_>>(), ["AA", "BB"]);
}

#[test]
fn library_indices_for_md5_are_case_insensitive_and_grouped() {
    let lib = Library::from_songs(vec![entry("AA", "one"), entry("bb", "two"), entry("aA", "three")]);
    assert_eq!(lib.indices_for_md5("aa"), [0, 2], "both spellings of the same chart group together");
    assert_eq!(lib.indices_for_md5("BB"), [1]);
    assert!(lib.indices_for_md5("cc").is_empty(), "an unknown md5 has no entries");
}

#[test]
fn an_empty_library_answers_every_query() {
    let lib = Library::default();
    assert!(lib.is_empty());
    assert_eq!(lib.len(), 0);
    assert!(lib.songs().is_empty());
    assert!(lib.indices_for_md5("AA").is_empty());
    assert_eq!(lib.md5s().count(), 0);
}

#[test]
fn only_the_json_extension_reads_as_bmson() {
    for name in ["a.bmson", "a.BMSON", "deep/dir/song.BmSoN"] {
        assert!(is_bmson(Path::new(name)), "{name} is bmson");
    }
    for name in ["a.bms", "a.bme", "a.pms", "a.json", "a"] {
        assert!(!is_bmson(Path::new(name)), "{name} is not bmson");
    }
}
