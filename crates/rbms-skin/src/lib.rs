//! Skin definition data and evaluation, with no rendering backend attached.
//!
//! The crate holds everything a JSON skin needs before a pixel is drawn: the timer registry and the
//! destination interpolator that animates objects, the property registry a skin reads game state
//! through, the serde mirror of the skin document, the loader that resolves the files it names, and
//! the sandboxed Lua evaluator for its expression-typed fields.
//!
//! Geometry and colour are this crate's own plain types rather than the renderer's, so the
//! dependency runs one way only: `rbms-render` uses `rbms-skin`, never the reverse. That keeps the
//! data and evaluation layers testable without a GPU, a window or a canvas.

#![forbid(unsafe_code)]

pub mod dst;
pub mod loader;
/// The sandboxed Lua evaluator for expression-typed skin fields. Compiled only with the `lua`
/// feature, which is on by default; a build without it rejects skins that carry Lua expressions
/// with [`SkinError::LuaUnavailable`] rather than silently treating them as false.
#[cfg(feature = "lua")]
pub mod lua;
pub mod model;
pub mod property;
pub mod resolve;
pub mod timer;

/// Everything that can go wrong while reading a skin document or evaluating one of its expressions.
///
/// Loading is deliberately fail-soft at the object level: a single unreadable image or a Lua
/// expression that will not compile drops that one object and warns, and only the variants below
/// abandon the whole skin so the caller can fall back to the built-in one.
#[derive(Debug, thiserror::Error)]
pub enum SkinError {
    /// The skin file, or a file it names, could not be read.
    #[error("{0}")]
    Read(#[source] std::io::Error),
    /// Neither the strict JSON parser nor the lenient json5 fallback could read the document.
    #[error("{path}:{line}:{column}: {message}")]
    Parse { path: String, line: usize, column: usize, message: String },
    /// The document is larger than the loader accepts, so it was never parsed.
    #[error("{path} is {actual} bytes, over the {limit} byte limit")]
    TooLarge { path: String, actual: u64, limit: u64 },
    /// A number in the document does not fit the field it was read into.
    #[error("{path}: {field} value {value} is out of range")]
    NumberRange { path: String, field: String, value: String },
    /// A resolved path left the skin root, whether through `..` segments or a symlink out of the
    /// tree. The skin root is the only directory a skin may read from.
    #[error("{0} resolves outside the skin root")]
    PathEscape(String),
    /// The document declares no skin type, so there is no screen to attach it to. The loader does
    /// not guess from the file name.
    #[error("the skin declares no type")]
    TypeMissing,
    /// The declared skin type is not one this build renders.
    #[error("skin type {0} is not supported")]
    TypeUnsupported(i32),
    /// A Lua expression failed to compile or raised while being evaluated.
    #[error("{expr}: {message}")]
    Lua { expr: String, message: String },
    /// A Lua expression ran past its instruction or memory budget and was cut off.
    #[error("{expr} exceeded its evaluation budget")]
    LuaBudget { expr: String },
    /// The skin needs Lua expression evaluation but this build was compiled without the feature.
    #[error("this build has no Lua support")]
    LuaUnavailable,
}

#[cfg(test)]
mod tests {
    use super::SkinError;

    #[test]
    fn parse_error_names_the_position() {
        let error = SkinError::Parse { path: "skin.json".to_owned(), line: 12, column: 4, message: "trailing comma".to_owned() };
        assert_eq!(error.to_string(), "skin.json:12:4: trailing comma");
    }

    #[test]
    fn path_escape_error_names_the_path() {
        let error = SkinError::PathEscape("../../etc/passwd".to_owned());
        assert_eq!(error.to_string(), "../../etc/passwd resolves outside the skin root");
    }

    #[test]
    fn read_error_keeps_the_io_source() {
        let error = SkinError::Read(std::io::Error::new(std::io::ErrorKind::NotFound, "no such file"));
        assert_eq!(error.to_string(), "no such file");
        assert!(std::error::Error::source(&error).is_some());
    }

    #[test]
    fn lua_budget_error_names_the_expression() {
        let error = SkinError::LuaBudget { expr: "while true do end".to_owned() };
        assert_eq!(error.to_string(), "while true do end exceeded its evaluation budget");
    }

    #[test]
    fn too_large_error_reports_both_sizes() {
        let error = SkinError::TooLarge { path: "skin.json".to_owned(), actual: 9, limit: 8 };
        assert_eq!(error.to_string(), "skin.json is 9 bytes, over the 8 byte limit");
    }
}
