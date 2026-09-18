//! Which blocks of a screen the selected document has taken over, and which the built-in layout
//! still draws.
//!
//! A document may replace a screen wholesale, sit over it, or -- the case these flags exist for --
//! stand in for named pieces of it while the rest of the built-in screen carries on. The score
//! screen had that contract first; a play document that draws its own note field and a browser
//! document that draws its own wheel need exactly the same thing, so the flags are one shape per
//! screen rather than one hard-coded pair per block.
//!
//! Nothing here decides anything. The application reads the document's `replace` list, checks that
//! every object the block needs actually compiled, and sets the flags; a screen handed the default
//! draws precisely what it drew before any document existed.

use crate::result::ResultContent;

/// The pieces of the play screen a document has taken over.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayContent {
    /// The note field: lane backgrounds, notes, the judgement line, key beams and bar lines.
    pub field: bool,
    /// The gauge bar and its reading.
    pub gauge: bool,
    /// The judgement pop-up, the combo and the fast/slow word.
    pub judge: bool,
    /// The EX, best and difference readings.
    pub score: bool,
    /// The per-judgement counters and the fast/slow tallies.
    pub counts: bool,
    /// The pace-maker graph column.
    pub graph: bool,
    /// The lane covers.
    pub cover: bool,
    /// The frame drawn around the field and the chart art.
    pub frame: bool,
}

/// The pieces of the song browser a document has taken over.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectContent {
    /// The row list, whose click rectangles the document answers for instead.
    pub list: bool,
    /// The detail pane beside it.
    pub detail: bool,
    /// The top bar and its buttons.
    pub topbar: bool,
    /// The option overlay.
    pub options: bool,
}

/// Every screen's replacement flags in one value, so one lookup answers whichever screen is being
/// drawn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScreenContent {
    pub play: PlayContent,
    pub select: SelectContent,
    pub result: ResultContent,
}
