use crate::*;

fn samples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/preview-demo")
}

fn idle() -> (AtomicUsize, AtomicBool) {
    (AtomicUsize::new(0), AtomicBool::new(false))
}

#[test]
fn is_chart_accepts_every_bms_extension_in_any_case() {
    for name in ["a.bms", "a.BME", "a.Bml", "a.pms", "deep/dir/song.bms"] {
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
fn scan_folder_reads_the_sample_charts_and_counts_them() {
    let (count, cancel) = idle();
    let songs = scan_folder(&samples_dir(), &count, &cancel);
    assert_eq!(songs.len(), 2, "the sample folder holds two charts");
    assert_eq!(count.load(Ordering::Relaxed), 2, "the progress counter matches the charts read");
    assert!(songs.iter().all(|s| !s.md5.is_empty()), "every entry carries the chart md5");
    assert!(songs.iter().all(|s| s.init_bpm > 0.0), "every entry carries the chart's initial BPM");
}

#[test]
fn scan_folder_sorts_by_lowercased_title() {
    let (count, cancel) = idle();
    let songs = scan_folder(&samples_dir(), &count, &cancel);
    let titles: Vec<String> = songs.iter().map(|s| s.title.to_lowercase()).collect();
    let mut sorted = titles.clone();
    sorted.sort();
    assert_eq!(titles, sorted, "entries come back title-ordered");
}

#[test]
fn scan_folder_of_a_missing_root_is_empty() {
    let (count, cancel) = idle();
    let missing = std::env::temp_dir().join(format!("rbms_library_missing_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&missing);
    assert!(scan_folder(&missing, &count, &cancel).is_empty(), "an unreadable folder contributes nothing");
    assert_eq!(count.load(Ordering::Relaxed), 0);
}

#[test]
fn a_cancelled_scan_reads_nothing() {
    let count = AtomicUsize::new(0);
    let cancel = AtomicBool::new(true);
    assert!(scan_folder(&samples_dir(), &count, &cancel).is_empty(), "a scan cancelled up front stops before the first read");
    assert_eq!(count.load(Ordering::Relaxed), 0);
}

#[test]
fn scan_folders_merges_every_folder_into_one_list() {
    let (count, cancel) = idle();
    let dir = samples_dir().to_string_lossy().to_string();
    let one = scan_folders(std::slice::from_ref(&dir), &count, &cancel);
    let (count2, cancel2) = idle();
    let twice = scan_folders(&[dir.clone(), dir], &count2, &cancel2);
    assert_eq!(twice.len(), one.len() * 2, "the library is the union of the folders, duplicates included");
    assert_eq!(count2.load(Ordering::Relaxed), count.load(Ordering::Relaxed) * 2);
}

#[test]
fn scan_folders_of_an_empty_folder_list_is_empty() {
    let (count, cancel) = idle();
    assert!(scan_folders(&[], &count, &cancel).is_empty());
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
