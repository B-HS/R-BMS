//! What becomes of the theme file on a read of it, in a folder made for the test.

use std::path::{Path, PathBuf};

use rbms_render::{Color, Theme, ThemeConfig};

use super::{PINNED_TEMPLATE, SUPERSEDED_TEMPLATES, THEME_TEMPLATE, ThemeFile, is_superseded_template, read_theme, read_theme_with, theme_path};

/// What starts a line of the template that shows a colour without setting it.
const COMMENTED_FIELD: &str = "    // ";

/// The fields of the theme file that set a layout or an opacity, not a colour, which the template
/// does not show.
const NOT_COLOURS: [&str; 3] = ["select_layout", "options_layout", "select_panel_alpha"];

/// A folder of the test's own, empty, for a theme file to be read from.
fn temp_folder(tag: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("rbms-theme-file-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("create the fixture folder");
    folder
}

/// A way to write a template that always fails, for a folder that cannot be written.
fn refused(_: &Path, _: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "the folder is read-only"))
}

/// The template with every colour it shows set: each commented line taken out of its comment.
fn template_with_every_colour_set() -> String {
    THEME_TEMPLATE
        .lines()
        .map(|line| line.strip_prefix(COMMENTED_FIELD).map_or_else(|| line.to_string(), |field| format!("    {field}")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The palette the builds before the achromatic one drew with, which is what their template set.
fn pinned_palette() -> Theme {
    ThemeConfig::parse(PINNED_TEMPLATE).resolve()
}

#[test]
fn the_theme_file_is_beside_the_settings_file() {
    assert_eq!(theme_path(Path::new("home/.config/rbms/settings.ron")), Path::new("home/.config/rbms").join("theme.ron"));
    assert_eq!(theme_path(Path::new("")), PathBuf::from("theme.ron"));
}

/// The template sets nothing, so a file that is still the template follows the defaults of
/// whichever build reads it.
#[test]
fn the_template_is_valid_ron_that_sets_no_colour() {
    let parsed: Result<ThemeConfig, _> = ron::from_str(THEME_TEMPLATE);
    let config = parsed.expect("the template is not valid RON");
    assert_eq!(config.resolve(), Theme::default());
    assert!(config.bg.is_none() && config.text.is_none() && config.warn.is_none(), "the template sets a colour");
    assert!(!is_superseded_template(THEME_TEMPLATE), "the template in force is taken for one that was given up");
}

/// What the template shows beside each field is the default it stands for: a player who takes a
/// line out of its comment without editing it changes nothing.
#[test]
fn every_colour_the_template_shows_is_the_default() {
    let set = template_with_every_colour_set();
    let parsed: Result<ThemeConfig, _> = ron::from_str(&set);
    let config = parsed.expect("the template with its lines taken out of their comments is not valid RON");
    assert!(config.bg.is_some() && config.title_dim.is_some() && config.warn.is_some(), "a line of the template was not taken out of its comment");
    assert_eq!(config.resolve(), Theme::default());
}

/// A colour the theme has but the template does not show would be one a player cannot find by
/// reading the file. With every line of the template taken out of its comment, the only fields of
/// the theme file left unset are the ones that are not colours.
#[test]
fn the_template_shows_every_colour_the_theme_file_can_set() {
    let config: ThemeConfig =
        ron::from_str(&template_with_every_colour_set()).expect("the template with its lines taken out of their comments is not valid RON");
    let unset: Vec<String> = format!("{config:#?}").lines().filter_map(|line| line.strip_prefix("    ")?.strip_suffix(": None,")).map(str::to_string).collect();
    assert_eq!(unset, NOT_COLOURS, "a colour of the theme file is not shown by the template, or a field that is not a colour was added");
}

#[test]
fn a_folder_with_no_theme_file_is_given_the_template() {
    let folder = temp_folder("none");
    let path = folder.join("nested").join("theme.ron");
    assert_eq!(read_theme(&path), (Theme::default(), ThemeFile::Written));
    assert_eq!(std::fs::read_to_string(&path).expect("read the written template"), THEME_TEMPLATE);
    assert_eq!(read_theme(&path), (Theme::default(), ThemeFile::Kept), "the template in force was written a second time");
    let _ = std::fs::remove_dir_all(&folder);
}

/// The reason the file is renewed at all: a template an earlier build wrote sets that build's
/// colours, and they are not this build's.
#[test]
fn a_template_that_was_given_up_set_colours_that_are_no_longer_the_defaults() {
    for template in SUPERSEDED_TEMPLATES {
        assert_ne!(ThemeConfig::parse(template).resolve(), Theme::default());
    }
}

/// A file that is still a template an earlier build wrote is replaced with the one in force,
/// whichever line ends it has and however many blank lines end it.
#[test]
fn an_untouched_template_of_an_earlier_build_is_replaced() {
    let folder = temp_folder("renewed");
    let path = folder.join("theme.ron");
    for template in SUPERSEDED_TEMPLATES {
        let windows = template.replace('\n', "\r\n");
        let trimmed = template.trim_end_matches('\n');
        let forms = [template.to_string(), windows.clone(), format!("{template}\n\n"), format!("{windows}\r\n"), trimmed.to_string()];
        for form in forms {
            std::fs::write(&path, &form).expect("write the earlier template");
            assert_eq!(read_theme(&path), (Theme::default(), ThemeFile::Renewed), "{form:?}");
            assert_eq!(std::fs::read_to_string(&path).expect("read the renewed file"), THEME_TEMPLATE);
        }
    }
    let _ = std::fs::remove_dir_all(&folder);
}

/// One changed value makes the file the player's: it is read as it stands, its other colours the
/// ones it names, and it is not written.
#[test]
fn a_template_the_player_edited_is_read_and_left_alone() {
    let folder = temp_folder("edited");
    let path = folder.join("theme.ron");
    let pinned = pinned_palette();
    let edits = [
        PINNED_TEMPLATE.replace("bg:            Some((8, 8, 14))", "bg:            Some((8, 8, 15))"),
        PINNED_TEMPLATE.replace("// window background", "// my background"),
        PINNED_TEMPLATE.replace("    warn:          Some((230, 180, 60)),\n", ""),
        format!(" {PINNED_TEMPLATE}"),
    ];
    for edited in edits {
        assert_ne!(edited, PINNED_TEMPLATE, "the fixture changes nothing");
        std::fs::write(&path, &edited).expect("write the edited file");
        let (theme, file) = read_theme(&path);
        assert_eq!(file, ThemeFile::Kept, "{edited:?}");
        assert_eq!(theme.panel, pinned.panel, "a colour the file names was not read");
        assert_eq!(theme.title_focus, pinned.title_focus, "a colour the file names was not read");
        assert_eq!(std::fs::read_to_string(&path).expect("read the file back"), edited, "the player's file was written");
    }
    std::fs::write(&path, PINNED_TEMPLATE.replace("bg:            Some((8, 8, 14))", "bg:            Some((1, 2, 3))")).expect("write the edited file");
    assert_eq!(read_theme(&path).0.bg, Color::rgb(1, 2, 3));
    let _ = std::fs::remove_dir_all(&folder);
}

/// A file that sets a few colours and nothing else keeps them over whatever the defaults are.
#[test]
fn a_file_of_the_players_own_sets_only_what_it_names() {
    let folder = temp_folder("partial");
    let path = folder.join("theme.ron");
    let own = "(bg: Some((1, 2, 3)), accent: Some((120, 205, 235)))";
    std::fs::write(&path, own).expect("write the file");
    let (theme, file) = read_theme(&path);
    assert_eq!(file, ThemeFile::Kept);
    assert_eq!((theme.bg, theme.accent), (Color::rgb(1, 2, 3), Color::rgb(120, 205, 235)));
    assert_eq!(theme.panel, Theme::default().panel);
    assert_eq!(std::fs::read_to_string(&path).expect("read the file back"), own);
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn a_broken_file_draws_the_defaults_and_is_left_alone() {
    let folder = temp_folder("broken");
    let path = folder.join("theme.ron");
    for broken in ["(bg: Some((8, 8", "not a theme at all", ""] {
        std::fs::write(&path, broken).expect("write the broken file");
        assert_eq!(read_theme(&path), (Theme::default(), ThemeFile::Kept), "{broken:?}");
        assert_eq!(std::fs::read_to_string(&path).expect("read the file back"), broken, "a file that could not be read was written over");
    }
    std::fs::write(&path, [0xff_u8, 0xfe, 0x00]).expect("write a file that is not text");
    assert_eq!(read_theme(&path), (Theme::default(), ThemeFile::Kept));
    assert_eq!(std::fs::read(&path).expect("read the file back"), [0xff, 0xfe, 0x00], "a file that is not text was written over");
    let _ = std::fs::remove_dir_all(&folder);
}

/// A folder that cannot be written does not stop the run or change what is drawn: the defaults
/// are in force whether or not the template reached the disk, and a file that was not replaced is
/// not reported as replaced.
#[test]
fn a_template_that_cannot_be_written_still_draws_the_defaults() {
    let folder = temp_folder("refused");
    let path = folder.join("theme.ron");
    assert_eq!(read_theme_with(&path, refused), (Theme::default(), ThemeFile::Kept));
    assert!(!path.exists(), "a refused write left a file");
    std::fs::write(&path, PINNED_TEMPLATE).expect("write the earlier template");
    assert_eq!(read_theme_with(&path, refused), (Theme::default(), ThemeFile::Kept));
    assert_eq!(std::fs::read_to_string(&path).expect("read the file back"), PINNED_TEMPLATE, "a refused write changed the file");
    let _ = std::fs::remove_dir_all(&folder);
}
