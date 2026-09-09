//! Long-note state machine: charge-note release deferral, hell-charge ticks and long-note modes.

use rbms_model::LnKind;
use serde::{Deserialize, Serialize};

use crate::Judge;
use crate::matcher::ScratchDir;

/// How long a hell-charge note takes to pay out one gauge tick while it is being held or missed
/// (`JudgeManager.java:81`, `hcnmduration`).
pub const HCN_GAUGE_TICK_US: i64 = 200_000;

/// Fraction of a normal judgment's gauge delta one hell-charge tick is worth
/// (`JudgeManager.java:313, 327`: `gauge.update(judge, 0.5f)`).
pub const HCN_GAUGE_TICK_RATE: f32 = 0.5;

/// Which flavour a long note with no chart-stated type takes on — the reference implementation's
/// `PlayerConfig.lnmode`, clamped to 0..2 (`PlayerConfig.java:109, 925`). A chart-stated flavour
/// always wins over this, so only [`LnKind::Undefined`] notes consult it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LnMode {
    #[default]
    LongNote,
    ChargeNote,
    HellChargeNote,
}

impl LnMode {
    /// Every mode, in reference id order.
    pub const ALL: [LnMode; 3] = [LnMode::LongNote, LnMode::ChargeNote, LnMode::HellChargeNote];

    /// The reference id this mode is stored under (`PlayerConfig.lnmode`).
    pub fn id(self) -> u8 {
        match self {
            LnMode::LongNote => 0,
            LnMode::ChargeNote => 1,
            LnMode::HellChargeNote => 2,
        }
    }

    /// Inverse of [`id`](Self::id); an id outside 0..2 yields `None`, which is where the reference
    /// clamps instead (`PlayerConfig.java:925`).
    pub fn from_id(id: u8) -> Option<LnMode> {
        LnMode::ALL.into_iter().find(|m| m.id() == id)
    }

    /// The concrete flavour an undefined chart note plays as under this mode.
    pub fn resolve(self) -> LnKind {
        match self {
            LnMode::LongNote => LnKind::Ln,
            LnMode::ChargeNote => LnKind::Cn,
            LnMode::HellChargeNote => LnKind::Hcn,
        }
    }
}

/// One lane's long-note state — the reference implementation's `JudgeManager.LaneState` fields
/// (`JudgeManager.java:855-872`) that track the note being held, a release awaiting confirmation,
/// which scratch direction is holding it and the hell-charge tick balance.
#[derive(Debug, Clone, Default)]
pub struct LaneHold {
    /// Index of the note being held, the reference's `processing` (which stores the pair's end
    /// note; this engine keeps head and end in one note, so it stores that note).
    pub note: Option<usize>,
    /// `lnstartJudge`: the judgment the head took, which a plain long note carries to its end.
    pub start_judge: Option<Judge>,
    /// `lnstartDuration`: the signed timing delta the head was hit with.
    pub start_delta_us: i64,
    /// `releasetime`: when a release was deferred, `None` for the reference's `Long.MIN_VALUE`.
    pub release_us: Option<i64>,
    /// `lnendJudge`: the judgment a deferred release will confirm with.
    pub end_judge: Option<Judge>,
    /// `sckey[sc]`: which scratch direction grabbed this note, `None` while unowned.
    pub owner: Option<ScratchDir>,
    /// `passing`: index of the hell-charge note the play head is currently inside.
    pub passing: Option<usize>,
    /// `mpassingcount`: hell-charge tick balance in microseconds.
    pub passing_us: i64,
    /// `inclease`: whether the hell-charge note is currently gaining rather than draining.
    pub increasing: bool,
}

impl LaneHold {
    /// Drop the held note together with every piece of deferred-release state that belongs to it,
    /// the three-field reset the reference performs at each release site
    /// (`JudgeManager.java:368-370, 517-520, 545-547, 619-624`).
    pub fn clear(&mut self) {
        self.note = None;
        self.start_judge = None;
        self.start_delta_us = 0;
        self.release_us = None;
        self.end_judge = None;
        self.owner = None;
    }
}

#[cfg(test)]
mod ln_tests {
    use super::*;

    #[test]
    fn ln_mode_ids_match_the_reference_clamp_range() {
        assert_eq!(LnMode::default(), LnMode::LongNote);
        for (id, mode) in LnMode::ALL.into_iter().enumerate() {
            assert_eq!(mode.id(), id as u8, "{mode:?}");
            assert_eq!(LnMode::from_id(mode.id()), Some(mode), "{mode:?}");
        }
        assert_eq!(LnMode::from_id(3), None, "PlayerConfig.java:925 clamps lnmode to 0..2");
    }

    #[test]
    fn ln_mode_resolves_to_the_matching_chart_flavour() {
        assert_eq!(LnMode::LongNote.resolve(), LnKind::Ln);
        assert_eq!(LnMode::ChargeNote.resolve(), LnKind::Cn);
        assert_eq!(LnMode::HellChargeNote.resolve(), LnKind::Hcn);
    }

    #[test]
    fn clearing_a_hold_drops_the_deferred_release_but_keeps_the_hell_charge_balance() {
        let mut hold = LaneHold {
            note: Some(3),
            start_judge: Some(Judge::Great),
            start_delta_us: 12,
            release_us: Some(99),
            end_judge: Some(Judge::Bad),
            owner: Some(ScratchDir::Backward),
            passing: Some(3),
            passing_us: 5_000,
            increasing: true,
        };
        hold.clear();
        assert_eq!(hold.note, None);
        assert_eq!(hold.start_judge, None);
        assert_eq!(hold.start_delta_us, 0);
        assert_eq!(hold.release_us, None);
        assert_eq!(hold.end_judge, None);
        assert_eq!(hold.owner, None);
        assert_eq!(hold.passing, Some(3), "the hell-charge pass is driven by the clock, not by the hold");
        assert_eq!(hold.passing_us, 5_000);
    }

    #[test]
    fn the_tick_constants_match_the_reference() {
        assert_eq!(HCN_GAUGE_TICK_US, 200_000, "JudgeManager.java:81 hcnmduration");
        assert_eq!(HCN_GAUGE_TICK_RATE, 0.5, "JudgeManager.java:313, 327");
    }
}
