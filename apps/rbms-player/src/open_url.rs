//! Handing a web address to the system's browser.
//!
//! Only an address [`rbms_download::browsable_url`] lets through is ever handed on: http or https
//! with a host, nothing a shell or the opener could read as anything but the address. The opener is
//! run as a program of its own with the address as its last argument, and no shell reads the line:
//! on Windows that is why it is `rundll32` and not `cmd /C start`, which reads `&`, `|`, `^` and `%`
//! in an address the way a table's links are full of.

#[cfg(not(test))]
use std::process::{Command, Stdio};

#[cfg(not(test))]
use crate::notify::{Level, notify};

/// The program that hands an address to the browser on macOS.
#[cfg(target_os = "macos")]
const OPENER: &str = "open";

/// The program that hands an address to the browser on Windows, and the entry point of it that
/// opens an address with whatever is registered for its scheme.
#[cfg(windows)]
const OPENER: &str = "rundll32";
#[cfg(windows)]
const WINDOWS_URL_HANDLER: &str = "url.dll,FileProtocolHandler";

/// The program that hands an address to the browser on every other system.
#[cfg(not(any(target_os = "macos", windows)))]
const OPENER: &str = "xdg-open";

/// The program and the arguments that hand `url` to this platform's browser. `url` has been through
/// [`rbms_download::browsable_url`]: it is the last argument, alone, and no shell reads it.
#[cfg(not(windows))]
pub(crate) fn opener_command(url: &str) -> (&'static str, Vec<String>) {
    (OPENER, vec![url.to_owned()])
}

/// The program and the arguments that hand `url` to this platform's browser. `url` has been through
/// [`rbms_download::browsable_url`]: it is the last argument, alone, and no shell reads it.
#[cfg(windows)]
pub(crate) fn opener_command(url: &str) -> (&'static str, Vec<String>) {
    (OPENER, vec![WINDOWS_URL_HANDLER.to_owned(), url.to_owned()])
}

/// Hand `raw` to `launch` as the opener's program and arguments, once it has been through
/// [`rbms_download::browsable_url`]. Answers whether it was handed on: an address the check refuses
/// never reaches `launch`.
fn hand_to(raw: &str, launch: impl FnOnce(&'static str, Vec<String>)) -> bool {
    let Some(url) = rbms_download::browsable_url(raw) else {
        return false;
    };
    let (program, arguments) = opener_command(&url);
    launch(program, arguments);
    true
}

/// Run the opener on a thread of its own and wait for it there, so the frame loop never waits on it
/// and it leaves no process behind. An opener that cannot be run, or that says it failed, is told.
#[cfg(not(test))]
fn launch(program: &'static str, arguments: Vec<String>) {
    std::thread::spawn(move || {
        let ran = Command::new(program).args(&arguments).stdin(Stdio::null()).spawn().and_then(|mut child| child.wait());
        match ran {
            Ok(status) if status.success() => {}
            Ok(status) => notify(Level::Warn, format!("could not open the browser: {program} {status}")),
            Err(error) => notify(Level::Warn, format!("could not open the browser: {error}")),
        }
    });
}

/// A test is never handed the system's opener: one that reaches for it fails there and then.
#[cfg(test)]
fn launch(program: &'static str, arguments: Vec<String>) {
    panic!("a test reached for the system's opener: {program} {arguments:?}");
}

/// Hand `raw` to the browser on a thread of its own, which waits for the opener so it leaves no
/// process behind. Nothing is done with an address `browsable_url` refuses.
pub(crate) fn open_in_browser(raw: &str) {
    hand_to(raw, launch);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A link of the kind a table lists, with the characters a shell would read.
    const SONG_PAGE: &str = "https://manbow.nothing.sh/event/event.cgi?action=More_def&num=94&event=146";

    #[test]
    fn the_opener_is_handed_the_url_alone() {
        let (program, arguments) = opener_command(SONG_PAGE);
        #[cfg(target_os = "macos")]
        assert_eq!((program, arguments.as_slice()), ("open", [SONG_PAGE.to_owned()].as_slice()));
        #[cfg(windows)]
        assert_eq!((program, arguments.as_slice()), ("rundll32", ["url.dll,FileProtocolHandler".to_owned(), SONG_PAGE.to_owned()].as_slice()));
        #[cfg(not(any(target_os = "macos", windows)))]
        assert_eq!((program, arguments.as_slice()), ("xdg-open", [SONG_PAGE.to_owned()].as_slice()));
    }

    #[test]
    fn an_address_that_is_not_http_is_never_handed_on() {
        for raw in ["javascript:alert(1)", "file:///etc/passwd", "data:text/html,hi", "-a Calculator", "", "https://", "https://a.test/x y"] {
            assert!(!hand_to(raw, |program, arguments| panic!("{raw:?} was handed to {program} {arguments:?}")), "{raw:?}");
        }
        let mut handed = None;
        assert!(hand_to(SONG_PAGE, |program, arguments| handed = Some((program, arguments))));
        assert_eq!(handed, Some(opener_command(SONG_PAGE)), "the address checked is the address handed on");
    }

    /// Opening an address the check refuses runs nothing, so it is safe to ask for from anywhere. In
    /// a test the opener fails the moment it is reached, so an address let through fails this.
    #[test]
    fn opening_a_refused_address_runs_nothing() {
        for raw in ["javascript:alert(1)", "file:///etc/passwd", "-a Calculator"] {
            open_in_browser(raw);
        }
    }
}
