//! A number a row holds, as a screen that sets one by hand counts it.
//!
//! A row that holds a number is set three ways: a key moves it one step, a slider is dragged along
//! it, and a figure is typed into it. All three count the row in whole steps of the smallest change
//! the row can hold ([`NumberSpec`]), so what a key reaches, what a drag lands on and what a typed
//! figure is rounded to are the same values, and stepping a row a thousand times adds no rounding
//! to it: the value stored is always a whole count divided by how many steps make one.
//!
//! Which rows of the settings table hold a number, what each is counted in and where it is stored
//! is said in `settings.rs` (`number_spec`, `number_steps`, `set_number_steps`). A screen with rows
//! of its own -- the offsets a skin declares -- makes a [`NumberSpec`] for each and is counted the
//! same way.

/// The steps of one row that holds a number: its two ends, how many steps make one of what the
/// row shows, and how what it shows is written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NumberSpec {
    /// The fewest steps the row holds.
    pub min: i64,
    /// The most steps the row holds.
    pub max: i64,
    /// How many steps make one of what the row shows: one for a row counted in whole numbers, ten
    /// for one shown to a tenth.
    pub per_one: i64,
    /// How many places after the point the row is shown to.
    pub decimals: u8,
    /// What is written after the number.
    pub unit: &'static str,
    /// Whether a number above nothing is written with its sign, as an offset is.
    pub signed: bool,
}

/// What a decimal point is typed as.
const DECIMAL_POINT: char = '.';

/// The two signs a typed number may begin with.
const SIGNS: [char; 2] = ['+', '-'];

impl NumberSpec {
    /// A row counted in whole numbers from `min` to `max`, written with `unit` after it.
    pub const fn whole(min: i64, max: i64, unit: &'static str) -> NumberSpec {
        NumberSpec { min, max, per_one: 1, decimals: 0, unit, signed: false }
    }

    /// A row shown to `decimals` places, `per_one` steps to each one of what it shows.
    pub const fn fine(min: i64, max: i64, per_one: i64, decimals: u8, unit: &'static str) -> NumberSpec {
        NumberSpec { min, max, per_one, decimals, unit, signed: false }
    }

    /// The same row with a number above nothing written with its sign.
    pub const fn with_sign(self) -> NumberSpec {
        NumberSpec { signed: true, ..self }
    }

    /// `steps` held to the row's two ends.
    pub fn clamp(&self, steps: i64) -> i64 {
        steps.clamp(self.min, self.max.max(self.min))
    }

    /// The step `by` steps away from `steps`, held to the row's two ends.
    pub fn moved(&self, steps: i64, by: i64) -> i64 {
        self.clamp(steps.saturating_add(by))
    }

    /// The step nearest to `shown`, a number in what the row shows, held to the row's two ends. A
    /// number that is none at all reads as the row's lower end.
    pub fn steps_of(&self, shown: f64) -> i64 {
        if shown.is_nan() {
            return self.clamp(self.min);
        }
        self.clamp((shown * self.per_one as f64).round() as i64)
    }

    /// What the row shows at `steps`, as a number.
    pub fn shown(&self, steps: i64) -> f64 {
        steps as f64 / self.per_one.max(1) as f64
    }

    /// The number the row shows at `steps`, without its unit and without the sign of a number above
    /// nothing: what a figure typed into the row starts as.
    pub fn plain(&self, steps: i64) -> String {
        format!("{:.*}", usize::from(self.decimals), self.shown(steps))
    }

    /// The number the row shows at `steps`, without its unit.
    pub fn figure(&self, steps: i64) -> String {
        if self.signed { format!("{:+.*}", usize::from(self.decimals), self.shown(steps)) } else { self.plain(steps) }
    }

    /// What the row shows at `steps`: the number and its unit.
    pub fn text(&self, steps: i64) -> String {
        format!("{}{}", self.figure(steps), self.unit)
    }

    /// The step a typed figure stands for: the nearest step to it, held to the row's two ends, or
    /// `None` when what was typed is not a number.
    ///
    /// A number is a sign or none, then digits with at most one decimal point among them. A unit
    /// typed after it -- a percent sign, `ms`, `x`, `px`, with or without a space before it -- is
    /// read past. Anything else is not a number: nothing at all, a sign alone, two points, digits
    /// after the unit.
    pub fn parse(&self, typed: &str) -> Option<i64> {
        let typed = typed.trim();
        let digits_from = usize::from(typed.starts_with(SIGNS));
        let number_end = typed[digits_from..].find(|ch: char| !ch.is_ascii_digit() && ch != DECIMAL_POINT).map_or(typed.len(), |at| at + digits_from);
        let (number, rest) = typed.split_at(number_end);
        if !number.contains(|ch: char| ch.is_ascii_digit()) || rest.contains(|ch: char| ch.is_ascii_digit() || ch == DECIMAL_POINT || SIGNS.contains(&ch)) {
            return None;
        }
        number.parse::<f64>().ok().filter(|value| value.is_finite()).map(|value| self.steps_of(value))
    }

    /// How far along the row `steps` is, from nothing at its lower end to one at its upper.
    pub fn fraction(&self, steps: i64) -> f64 {
        let span = self.max.saturating_sub(self.min);
        if span <= 0 {
            return 0.0;
        }
        self.clamp(steps).saturating_sub(self.min) as f64 / span as f64
    }

    /// The step nearest to `fraction` of the way along the row. A fraction outside the row lands on
    /// the end it is past, and one that is no number on the lower end.
    pub fn at_fraction(&self, fraction: f64) -> i64 {
        let span = self.max.saturating_sub(self.min).max(0);
        let along = if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) };
        self.clamp(self.min.saturating_add((along * span as f64).round() as i64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERCENT: NumberSpec = NumberSpec::whole(0, 100, "%");
    const TENTHS: NumberSpec = NumberSpec::fine(0, 1000, 10, 1, "%");
    const OFFSET: NumberSpec = NumberSpec::whole(-200, 200, " MS").with_sign();
    const SPEED: NumberSpec = NumberSpec::fine(1, 2000, 100, 2, "");

    #[test]
    fn a_row_is_written_with_its_places_its_sign_and_its_unit() {
        assert_eq!(PERCENT.text(35), "35%");
        assert_eq!(TENTHS.text(253), "25.3%");
        assert_eq!(TENTHS.text(1000), "100.0%");
        assert_eq!(OFFSET.text(0), "+0 MS");
        assert_eq!(OFFSET.text(-12), "-12 MS");
        assert_eq!(OFFSET.plain(12), "12", "what is typed over carries no sign of its own");
        assert_eq!(SPEED.text(225), "2.25");
        assert_eq!(SPEED.figure(1), "0.01");
    }

    /// Every step of a row reads back as itself from what the row shows for it, so stepping a row
    /// and typing what it shows are the same value.
    #[test]
    fn every_step_reads_back_from_what_it_shows() {
        for spec in [PERCENT, TENTHS, OFFSET, SPEED] {
            for steps in spec.min..=spec.max {
                assert_eq!(spec.steps_of(spec.shown(steps)), steps, "{spec:?}");
                assert_eq!(spec.parse(&spec.text(steps)), Some(steps), "{spec:?} typed as {}", spec.text(steps));
                assert_eq!(spec.parse(&spec.plain(steps)), Some(steps), "{spec:?} typed as {}", spec.plain(steps));
            }
        }
    }

    #[test]
    fn a_step_stops_at_the_ends() {
        assert_eq!(PERCENT.moved(99, 10), 100);
        assert_eq!(PERCENT.moved(3, -10), 0);
        assert_eq!(OFFSET.moved(0, -1), -1);
        assert_eq!(OFFSET.moved(200, i64::MAX), 200);
        assert_eq!(OFFSET.moved(-200, i64::MIN), -200);
    }

    #[test]
    fn a_typed_figure_is_rounded_to_the_nearest_step_and_held_to_the_ends() {
        assert_eq!(TENTHS.parse("25.34"), Some(253));
        assert_eq!(TENTHS.parse("25.35"), Some(254));
        assert_eq!(PERCENT.parse("49.6"), Some(50));
        assert_eq!(PERCENT.parse("250"), Some(100), "past the upper end");
        assert_eq!(PERCENT.parse("-3"), Some(0), "past the lower end");
        assert_eq!(OFFSET.parse("-12"), Some(-12));
        assert_eq!(OFFSET.parse("+7"), Some(7));
        assert_eq!(SPEED.parse(".5"), Some(50));
        assert_eq!(SPEED.parse("3."), Some(300));
        assert_eq!(SPEED.parse("999999999999999999999999"), Some(2000));
    }

    #[test]
    fn a_unit_typed_after_a_figure_is_read_past() {
        assert_eq!(PERCENT.parse("40%"), Some(40));
        assert_eq!(PERCENT.parse(" 40 % "), Some(40));
        assert_eq!(OFFSET.parse("-5ms"), Some(-5));
        assert_eq!(OFFSET.parse("-5 MS"), Some(-5));
        assert_eq!(SPEED.parse("2.5x"), Some(250));
        assert_eq!(OFFSET.parse("12px"), Some(12));
    }

    #[test]
    fn what_is_not_a_number_is_refused() {
        for typed in ["", " ", "-", "+", ".", "-.", "abc", "x", "1.2.3", "--5", "5-", "1e5", "5 5", "nan", "inf", "%", "12ms3"] {
            assert_eq!(PERCENT.parse(typed), None, "{typed:?} was read as a number");
        }
    }

    #[test]
    fn a_place_along_the_row_and_a_step_are_told_from_one_another() {
        assert_eq!(PERCENT.at_fraction(0.0), 0);
        assert_eq!(PERCENT.at_fraction(1.0), 100);
        assert_eq!(PERCENT.at_fraction(0.504), 50);
        assert_eq!(PERCENT.at_fraction(-3.0), 0);
        assert_eq!(PERCENT.at_fraction(7.0), 100);
        assert_eq!(PERCENT.at_fraction(f64::NAN), 0);
        assert_eq!(OFFSET.at_fraction(0.5), 0);
        for steps in OFFSET.min..=OFFSET.max {
            assert_eq!(OFFSET.at_fraction(OFFSET.fraction(steps)), steps);
        }
        assert_eq!(NumberSpec::whole(5, 5, "").fraction(5), 0.0, "a row of one step has nowhere to be along");
        assert_eq!(NumberSpec::whole(5, 5, "").at_fraction(1.0), 5);
    }
}
