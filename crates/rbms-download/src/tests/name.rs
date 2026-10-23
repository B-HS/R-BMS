use std::fs;

use super::{Fixture, MD5};
use crate::name::is_safe_component;
use crate::{DownloadError, safe_folder_name, settle};

const ID: &str = "01234567";

#[test]
fn a_title_becomes_a_folder_name_every_platform_takes() {
    assert_eq!(safe_folder_name("Song", MD5), format!("Song [{ID}]"));
    assert_eq!(safe_folder_name("a<b>c:d\"e/f\\g|h?i*j", MD5), format!("a_b_c_d_e_f_g_h_i_j [{ID}]"));
    assert_eq!(safe_folder_name("tab\there\u{7}bell", MD5), format!("tab_here_bell [{ID}]"));
    assert_eq!(safe_folder_name("  ..Song..  ", MD5), format!("Song [{ID}]"));
    assert_eq!(safe_folder_name("\u{66f2}\u{540d}", MD5), format!("\u{66f2}\u{540d} [{ID}]"));
}

#[test]
fn a_title_with_nothing_in_it_gives_the_md5_alone() {
    for title in ["", " ", "...", " . . ", "\u{3000}"] {
        assert_eq!(safe_folder_name(title, MD5), ID, "{title:?}");
    }
}

#[test]
fn the_name_of_a_windows_device_gives_the_md5_alone() {
    for title in ["CON", "nul", "Com1", "LPT9", "aux.txt", "prn.tar.gz", "CON "] {
        assert_eq!(safe_folder_name(title, MD5), ID, "{title:?}");
    }
    for title in ["COM10", "Console", "COM0", "Nullable", "a.con"] {
        assert_eq!(safe_folder_name(title, MD5), format!("{title} [{ID}]"), "{title:?}");
    }
}

#[test]
fn a_long_title_is_cut_by_characters_and_by_bytes() {
    let a_hundred_letters = "a".repeat(100);
    assert_eq!(safe_folder_name(&a_hundred_letters, MD5), format!("{} [{ID}]", "a".repeat(64)));
    let a_hundred_wide = "\u{66f2}".repeat(100);
    assert_eq!(safe_folder_name(&a_hundred_wide, MD5), format!("{} [{ID}]", "\u{66f2}".repeat(64)));
    let a_hundred_four_byte = "\u{1d11e}".repeat(100);
    assert_eq!(safe_folder_name(&a_hundred_four_byte, MD5), format!("{} [{ID}]", "\u{1d11e}".repeat(50)));
}

#[test]
fn what_the_cut_leaves_at_the_end_is_trimmed_too() {
    let title = format!("{} b", "a".repeat(63));
    assert_eq!(safe_folder_name(&title, MD5), format!("{} [{ID}]", "a".repeat(63)));
}

#[test]
fn a_short_or_missing_md5_still_gives_a_name() {
    assert_eq!(safe_folder_name("Song", "abc"), "Song [abc]");
    assert_eq!(safe_folder_name("Song", ""), "Song");
    assert_eq!(safe_folder_name("", ""), "chart");
    assert_eq!(safe_folder_name("Song", "a/b"), "Song [a_b]");
}

#[test]
fn every_folder_name_made_is_a_plain_one() {
    for title in ["Song", "a<b", "CON", "", "...", "tab\there", "x.", "\u{66f2}", &"z".repeat(300)] {
        for md5 in [MD5, "", "ab"] {
            let name = safe_folder_name(title, md5);
            assert!(is_safe_component(&name), "{title:?} / {md5:?} became {name:?}");
        }
    }
}

#[test]
fn a_staging_folder_moves_to_the_name_asked_for() {
    let fixture = Fixture::new("settle-moves");
    let staging = fixture.path("staging");
    fs::create_dir_all(&staging).expect("make the staging folder");
    fs::write(staging.join("a.bms"), b"chart").expect("write into the staging folder");
    let settled = settle(&staging, &fixture.path("songs"), "Song [01234567]").expect("settle the folder");
    assert_eq!(settled, fixture.path("songs").join("Song [01234567]"));
    assert_eq!(fs::read(settled.join("a.bms")).expect("the chart moved"), b"chart");
    assert!(!staging.exists());
}

#[test]
fn a_folder_that_is_there_is_never_overwritten() {
    let fixture = Fixture::new("settle-numbered");
    let parent = fixture.path("songs");
    fs::create_dir_all(parent.join("Song")).expect("make the folder that is there");
    fs::write(parent.join("Song").join("mine.bms"), b"mine").expect("write into it");
    fs::write(parent.join("Other"), b"a file").expect("make a file with a name that is wanted");
    for (index, (name, expected)) in [("Song", "Song (2)"), ("Song", "Song (3)"), ("Other", "Other (2)")].into_iter().enumerate() {
        let staging = fixture.path(&format!("staging-{index}"));
        fs::create_dir_all(&staging).expect("make a staging folder");
        fs::write(staging.join("new.bms"), format!("new {index}")).expect("write into it");
        let settled = settle(&staging, &parent, name).expect("settle the folder");
        assert_eq!(settled, parent.join(expected));
        assert_eq!(fs::read(settled.join("new.bms")).expect("the chart moved"), format!("new {index}").into_bytes());
    }
    assert_eq!(fs::read(parent.join("Song").join("mine.bms")).expect("the first folder is as it was"), b"mine");
    assert_eq!(fs::read(parent.join("Other")).expect("the file is as it was"), b"a file");
}

#[test]
fn a_name_that_is_not_a_plain_folder_name_is_refused() {
    let fixture = Fixture::new("settle-refuses");
    let staging = fixture.path("staging");
    fs::create_dir_all(&staging).expect("make the staging folder");
    for name in ["", ".", "..", "../x", "a/b", "a\\b", "C:", "CON", "trailing ", "trailing."] {
        assert!(matches!(settle(&staging, &fixture.path("songs"), name), Err(DownloadError::UnsafeName(_))), "{name:?}");
    }
    assert!(staging.exists());
    assert!(!fixture.path("songs").exists());
}

#[test]
fn a_staging_folder_that_is_not_there_is_an_error_and_makes_nothing() {
    let fixture = Fixture::new("settle-missing");
    let settled = settle(&fixture.path("staging"), &fixture.path("songs"), "Song");
    assert!(matches!(settled, Err(DownloadError::Io(_))));
    assert!(!fixture.path("songs").join("Song").exists());
}
