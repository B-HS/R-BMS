//! Fixed words of the song select stage, by language: its key guide, its scene, the filter panel, the overlays a skin's browser gets, the
//! modal over a chart the library does not hold, the folders of the tree and the notices and status lines it raises.
//!
//! One module per part of the stage, each a catalog of its own. A word a chart, a folder, a table or the score server supplies is not
//! here, and neither is a term every language keeps as it is (a judgement, a gauge, a lamp, an option, a key). The words the ranking
//! overlay composes itself are in [`super::ranking`].

pub(crate) mod download_modal;
pub(crate) mod filter;
pub(crate) mod guide;
pub(crate) mod notices;
pub(crate) mod overlay;
pub(crate) mod scene;
pub(crate) mod tree;
