//! The UI theme file, `theme.ron`, beside the settings file.
//!
//! The file names the colours of the app's own screens and menus, each of them optional: a colour
//! it does not name is the app's default ([`rbms_render::Theme`]). What the app writes there when
//! there is no file is a template that names none -- every line of it is a comment showing a
//! default and what it colours -- so a player who never edits the file follows the defaults of
//! whichever build they run.
//!
//! The first template did name every colour, which pinned the defaults of the build that wrote it:
//! a later build with other defaults drew the earlier ones for anyone who had run the earlier
//! build once. A file that is still one of those templates, to the character, is therefore taken
//! for what it is -- nothing the player chose -- and replaced with the current template. Line ends
//! of either kind and blank lines at the end are not counted as a difference, since a checkout or
//! an editor that only opened the file may have changed them. A file that differs in anything else
//! is the player's own and is read as it stands, and never written.
//!
//! Nothing here fails the run: a file that cannot be read or parsed leaves the defaults in place,
//! and a template that cannot be written is only not written.

use std::path::{Path, PathBuf};

use rbms_render::{Theme, ThemeConfig};

#[cfg(test)]
mod tests;

/// The name of the theme file, in the folder the settings file is in.
const THEME_FILE: &str = "theme.ron";

/// The template written when there is no theme file: every colour as a comment, with its default
/// and what it colours, so the file pins nothing. See `docs/theme.md`.
pub(crate) const THEME_TEMPLATE: &str = "\
// rbms UI theme -- colours of the app's own screens and menus (0-255 per channel).
// Every colour below is commented out, so the app's defaults apply and follow the app when a
// later version changes them. To set a colour, remove the `//` at the start of its line and edit
// the value. The screens a skin draws, and the note field, are themed by the skin, not here.
(
    // bg:            Some((0, 0, 0)),       // window background
    // topbar:        Some((22, 22, 22)),    // top header bar
    // panel:         Some((22, 22, 22)),    // panel fill
    // panel_hi:      Some((38, 38, 38)),    // raised panel / button
    // divider:       Some((60, 60, 60)),    // dividers / outlines
    // text:          Some((242, 242, 242)), // primary text
    // text_dim:      Some((184, 184, 184)), // secondary text
    // text_muted:    Some((144, 144, 144)), // muted hints
    // accent:        Some((255, 255, 255)), // headings / counts / links / prompts
    // focus:         Some((255, 255, 255)), // focus ring / selection rails / caret / slider grip
    // title_focus:   Some((255, 255, 255)), // what is written on the focused row or the chosen tab
    // title_dim:     Some((200, 200, 200)), // unfocused row title
    // row_even:      Some((22, 22, 22)),    // even list row
    // row_odd:       Some((30, 30, 30)),    // odd list row
    // row_folder:    Some((38, 38, 38)),    // folder list row
    // row_focus:     Some((60, 60, 60)),    // focused list row
    // lamp_default:  Some((68, 68, 68)),    // clear lamp with no record yet
    // button:        Some((80, 80, 80)),    // chosen tab / focused menu row
    // button_active: Some((50, 50, 50)),    // active button, search box
    // good:          Some((200, 200, 200)), // progress bars
    // warn:          Some((230, 180, 60)),  // cautions
)
";

/// The template the builds before the achromatic palette wrote, which named every colour of that
/// palette and so kept it in force after the defaults changed.
const PINNED_TEMPLATE: &str = "\
// rbms UI theme — colours for the song-select / result / menu chrome (0-255 per channel).
// Omit any line to keep its default. The in-play note field is themed by the skin, not here.
(
    bg:            Some((8, 8, 14)),      // window background
    topbar:        Some((18, 18, 30)),    // top header bar
    panel:         Some((14, 16, 26)),    // panel fill
    panel_hi:      Some((22, 24, 36)),    // raised panel / button
    divider:       Some((40, 42, 58)),    // dividers / outlines
    text:          Some((235, 235, 235)), // primary text
    text_dim:      Some((170, 170, 185)), // secondary text
    text_muted:    Some((120, 124, 140)), // muted hints
    accent:        Some((120, 205, 235)), // accents / counts / sort label
    focus:         Some((120, 200, 240)), // focus ring / selection rails
    title_focus:   Some((244, 236, 156)), // focused row title
    title_dim:     Some((190, 186, 150)), // unfocused row title
    row_even:      Some((22, 24, 34)),
    row_odd:       Some((27, 29, 42)),
    row_folder:    Some((40, 44, 30)),
    row_focus:     Some((50, 56, 82)),
    lamp_default:  Some((44, 44, 54)),     // clear-lamp with no record yet
    button:        Some((70, 80, 120)),    // selected tab / button fill
    button_active: Some((40, 56, 80)),     // active/toggled button, search box
    good:          Some((90, 200, 230)),   // progress / score-graph bars
    warn:          Some((230, 180, 60)),
)
";

/// Every template an earlier build wrote that named colours, each of which a file may still be.
/// A template that is given up for another is added here as it was shipped, never edited.
const SUPERSEDED_TEMPLATES: [&str; 1] = [PINNED_TEMPLATE];

/// What became of the theme file on a read of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemeFile {
    /// There was none, and the template was written.
    Written,
    /// It was a template an earlier build wrote, untouched, and the current one replaced it.
    Renewed,
    /// It was left as it is: the player's own, one that could not be read, or a missing file or an
    /// earlier template that the template in force could not be written over.
    Kept,
}

/// Where the theme file is for the settings file at `settings_path`.
pub(crate) fn theme_path(settings_path: &Path) -> PathBuf {
    settings_path.parent().map(|directory| directory.join(THEME_FILE)).unwrap_or_else(|| PathBuf::from(THEME_FILE))
}

/// `text` as it is compared with a template: line ends of one kind, and no blank lines at its end.
fn settled(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

/// Whether `text` is a template an earlier build wrote and nobody has edited since.
fn is_superseded_template(text: &str) -> bool {
    let text = settled(text);
    SUPERSEDED_TEMPLATES.iter().any(|template| settled(template) == text)
}

/// The palette the theme file at `path` gives and what became of the file, `write` being how a
/// template is put there. A template that could not be written changes nothing of what is drawn:
/// the palette is the defaults either way, and the file is reported as kept since it is.
fn read_theme_with(path: &Path, write: impl Fn(&Path, &str) -> std::io::Result<()>) -> (Theme, ThemeFile) {
    let file = match std::fs::read_to_string(path) {
        Ok(text) if is_superseded_template(&text) => ThemeFile::Renewed,
        Ok(text) => return (ThemeConfig::parse(&text).resolve(), ThemeFile::Kept),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ThemeFile::Written,
        Err(_) => return (Theme::default(), ThemeFile::Kept),
    };
    let file = if write(path, THEME_TEMPLATE).is_ok() { file } else { ThemeFile::Kept };
    (ThemeConfig::parse(THEME_TEMPLATE).resolve(), file)
}

/// The palette the theme file at `path` gives, and what became of the file.
pub(crate) fn read_theme(path: &Path) -> (Theme, ThemeFile) {
    read_theme_with(path, crate::write_atomic)
}

/// Read the UI theme beside the settings file and put it in force, writing the template there
/// first when there is no file, or only a template an earlier build wrote.
pub(crate) fn load_theme(settings_path: &Path) {
    let path = theme_path(settings_path);
    let (theme, file) = read_theme(&path);
    rbms_render::set_theme(theme);
    match file {
        ThemeFile::Renewed => println!("theme: {} (the template of an earlier version was replaced; the colours follow the defaults)", path.display()),
        ThemeFile::Written | ThemeFile::Kept => println!("theme: {}", path.display()),
    }
}
