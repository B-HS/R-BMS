//! Catalogs of the player's fixed words, one module per area of the screens.
//!
//! Each module holds the `rbms_model::translations!` catalogs of one area, read in the language
//! the frame thread has installed ([`rbms_render::text::language`], settled by
//! [`crate::app_language`]). Words a skin, a chart or the score server supplies are not here, and
//! neither are the terms every language keeps as they are (judgements, gauges, lamps, options).
//! The terms and their translations are fixed in `docs/reference/2026-10-24-i18n-glossary.md`.

pub(crate) mod layout_play;
pub(crate) mod menus;
pub(crate) mod notices;
pub(crate) mod ranking;
pub(crate) mod select;
pub(crate) mod settings;
