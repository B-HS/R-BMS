//! The window the player opens: how large it is and whether it covers the screen.
//!
//! Both are stored under separator-free tokens, so a reworded label leaves an existing file alone,
//! and an unknown token reads back as the shipped choice instead of failing the whole document.

use serde::{Deserialize, Deserializer, Serializer};

/// How many window sizes the RESOLUTION row offers.
pub const WINDOW_RESOLUTION_COUNT: usize = 5;

/// How many window modes the WINDOW MODE row offers.
pub const WINDOW_MODE_COUNT: usize = 2;

/// The inner size the window opens at, in logical pixels. Every entry is 16:9, the proportion the
/// built-in screens are laid out in.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WindowResolution {
    #[default]
    Hd720,
    Hd900,
    FullHd1080,
    QuadHd1440,
    UltraHd2160,
}

impl WindowResolution {
    /// Every size, smallest first, in the order the row steps through them.
    pub const ALL: [WindowResolution; WINDOW_RESOLUTION_COUNT] =
        [WindowResolution::Hd720, WindowResolution::Hd900, WindowResolution::FullHd1080, WindowResolution::QuadHd1440, WindowResolution::UltraHd2160];

    /// Width and height of the window's inner area, in logical pixels.
    pub fn size(self) -> (u32, u32) {
        match self {
            WindowResolution::Hd720 => (1280, 720),
            WindowResolution::Hd900 => (1600, 900),
            WindowResolution::FullHd1080 => (1920, 1080),
            WindowResolution::QuadHd1440 => (2560, 1440),
            WindowResolution::UltraHd2160 => (3840, 2160),
        }
    }

    /// Name the row shows.
    pub fn label(self) -> &'static str {
        match self {
            WindowResolution::Hd720 => "1280x720",
            WindowResolution::Hd900 => "1600x900",
            WindowResolution::FullHd1080 => "1920x1080",
            WindowResolution::QuadHd1440 => "2560x1440",
            WindowResolution::UltraHd2160 => "3840x2160",
        }
    }

    /// Token the choice is stored under.
    pub fn token(self) -> &'static str {
        self.label()
    }

    /// The size a stored token names, falling back to the shipped one.
    pub fn from_token(token: &str) -> WindowResolution {
        WindowResolution::ALL.into_iter().find(|entry| entry.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }
}

/// Whether the window is an ordinary window or covers the whole screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WindowMode {
    #[default]
    Windowed,
    /// A decoration-free window over the whole monitor the window is on, with no video mode change.
    Borderless,
}

impl WindowMode {
    /// Every mode, in the order the row steps through them.
    pub const ALL: [WindowMode; WINDOW_MODE_COUNT] = [WindowMode::Windowed, WindowMode::Borderless];

    /// Name the row shows.
    pub fn label(self) -> &'static str {
        match self {
            WindowMode::Windowed => "WINDOWED",
            WindowMode::Borderless => "BORDERLESS",
        }
    }

    /// Token the choice is stored under.
    pub fn token(self) -> &'static str {
        self.label()
    }

    /// The mode a stored token names, falling back to the windowed one.
    pub fn from_token(token: &str) -> WindowMode {
        WindowMode::ALL.into_iter().find(|entry| entry.token().eq_ignore_ascii_case(token.trim())).unwrap_or_default()
    }
}

/// Labels the RESOLUTION row shows, one per entry of [`WindowResolution::ALL`].
pub const WINDOW_RESOLUTION_LABELS: &[&str] = &["1280x720", "1600x900", "1920x1080", "2560x1440", "3840x2160"];

/// Labels the WINDOW MODE row shows, one per entry of [`WindowMode::ALL`].
pub const WINDOW_MODE_LABELS: &[&str] = &["WINDOWED", "BORDERLESS"];

/// Stores [`WindowResolution`] as its token.
pub mod window_resolution_token {
    use super::{Deserialize, Deserializer, Serializer, WindowResolution};

    /// Writes the token.
    pub fn serialize<S: Serializer>(value: &WindowResolution, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    /// Reads the token back.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<WindowResolution, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(WindowResolution::from_token(&token))
    }
}

/// Stores [`WindowMode`] as its token.
pub mod window_mode_token {
    use super::{Deserialize, Deserializer, Serializer, WindowMode};

    /// Writes the token.
    pub fn serialize<S: Serializer>(value: &WindowMode, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.token())
    }

    /// Reads the token back.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<WindowMode, D::Error> {
        let token = String::deserialize(deserializer)?;
        Ok(WindowMode::from_token(&token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_labels_follow_the_choices_in_order() {
        assert_eq!(WindowResolution::ALL.map(WindowResolution::label).to_vec(), WINDOW_RESOLUTION_LABELS);
        assert_eq!(WindowMode::ALL.map(WindowMode::label).to_vec(), WINDOW_MODE_LABELS);
    }

    #[test]
    fn every_size_is_sixteen_by_nine_and_reads_back_from_its_token() {
        for entry in WindowResolution::ALL {
            let (width, height) = entry.size();
            assert_eq!(width * 9, height * 16, "{entry:?} is not 16:9");
            assert_eq!(WindowResolution::from_token(entry.token()), entry);
            assert_eq!(entry.label(), format!("{width}x{height}"));
        }
        for entry in WindowMode::ALL {
            assert_eq!(WindowMode::from_token(entry.token()), entry);
        }
    }

    #[test]
    fn the_shipped_window_is_the_1280x720_one() {
        assert_eq!(WindowResolution::default().size(), (1280, 720));
        assert_eq!(WindowMode::default(), WindowMode::Windowed);
    }

    #[test]
    fn an_unknown_token_falls_back_to_the_shipped_choice() {
        assert_eq!(WindowResolution::from_token("nonsense"), WindowResolution::Hd720);
        assert_eq!(WindowMode::from_token("nonsense"), WindowMode::Windowed);
        assert_eq!(WindowResolution::from_token(" 1920X1080 "), WindowResolution::FullHd1080, "the token is trimmed and case-blind");
    }
}
