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
//! of its own -- the offsets a skin declares, the terms a chart is practised under -- makes a
//! [`NumberSpec`] for each and is counted the same way. Such a row's two ends may move while the
//! screen is up: its screen makes the [`NumberSpec`] anew whenever it asks what the row holds.
//!
//! A row may hold a length of time ([`NumberSpec::as_clock`]). It is counted like any other, in
//! steps of a part of a second, and differs only in how it is written and typed: as minutes and
//! seconds.

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
    /// Whether the number is a length of time in seconds, written and typed as minutes and seconds.
    pub clock: bool,
}

/// What a decimal point is typed as.
const DECIMAL_POINT: char = '.';

/// The sign a number above nothing is written with when its row asks for one, the sign of a number
/// below nothing, and the two as what a typed number may begin with.
const PLUS: char = '+';
const MINUS: char = '-';
const SIGNS: [char; 2] = [PLUS, MINUS];

/// What stands between the minutes and the seconds of a length of time.
const CLOCK_SEPARATOR: char = ':';

/// How many seconds make a minute.
const SECONDS_PER_MINUTE: i64 = 60;

/// What each place after the point divides a second by.
const PLACE_BASE: i64 = 10;

impl NumberSpec {
    /// A row counted in whole numbers from `min` to `max`, written with `unit` after it.
    pub const fn whole(min: i64, max: i64, unit: &'static str) -> NumberSpec {
        NumberSpec { min, max, per_one: 1, decimals: 0, unit, signed: false, clock: false }
    }

    /// A row shown to `decimals` places, `per_one` steps to each one of what it shows.
    pub const fn fine(min: i64, max: i64, per_one: i64, decimals: u8, unit: &'static str) -> NumberSpec {
        NumberSpec { min, max, per_one, decimals, unit, signed: false, clock: false }
    }

    /// The same row with a number above nothing written with its sign.
    pub const fn with_sign(self) -> NumberSpec {
        NumberSpec { signed: true, ..self }
    }

    /// The same row holding a length of time: what it shows is seconds, written and typed as minutes
    /// and seconds.
    pub const fn as_clock(self) -> NumberSpec {
        NumberSpec { clock: true, ..self }
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
        if self.clock {
            return self.clock_figure(steps);
        }
        format!("{:.*}", usize::from(self.decimals), self.shown(steps))
    }

    /// The number the row shows at `steps`, without its unit.
    pub fn figure(&self, steps: i64) -> String {
        match (self.signed, self.clock) {
            (true, true) if steps >= 0 => format!("{PLUS}{}", self.clock_figure(steps)),
            (true, false) => format!("{:+.*}", usize::from(self.decimals), self.shown(steps)),
            _ => self.plain(steps),
        }
    }

    /// The length of time the row shows at `steps`, as minutes and seconds: the seconds to two
    /// digits and to the places the row is shown to, so they never read as sixty.
    fn clock_figure(&self, steps: i64) -> String {
        let places = usize::from(self.decimals);
        let per_second = PLACE_BASE.saturating_pow(u32::from(self.decimals));
        let per_minute = SECONDS_PER_MINUTE.saturating_mul(per_second);
        let parts = (self.shown(steps).abs() * per_second as f64).round() as i64;
        let (minutes, seconds, rest) = (parts / per_minute, parts % per_minute / per_second, parts % per_second);
        let sign = if steps < 0 { MINUS.to_string() } else { String::new() };
        if places == 0 {
            return format!("{sign}{minutes}{CLOCK_SEPARATOR}{seconds:02}");
        }
        format!("{sign}{minutes}{CLOCK_SEPARATOR}{seconds:02}{DECIMAL_POINT}{rest:0places$}")
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
    ///
    /// A length of time is typed as seconds, like any other number, or as it is written: a sign or
    /// none, whole minutes, a colon, and the seconds. Seconds past a minute are counted on into the
    /// next one. A row that is no length of time takes no colon.
    pub fn parse(&self, typed: &str) -> Option<i64> {
        let typed = typed.trim();
        let seconds = match typed.split_once(CLOCK_SEPARATOR).filter(|_| self.clock) {
            Some((minutes, seconds)) => clock_seconds(minutes, seconds),
            None => typed_number(typed),
        };
        seconds.map(|value| self.steps_of(value))
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

/// The number `typed` stands for, a unit after it read past, or `None` when it is not a number.
fn typed_number(typed: &str) -> Option<f64> {
    let digits_from = usize::from(typed.starts_with(SIGNS));
    let number_end = typed[digits_from..].find(|ch: char| !ch.is_ascii_digit() && ch != DECIMAL_POINT).map_or(typed.len(), |at| at + digits_from);
    let (number, rest) = typed.split_at(number_end);
    if !number.contains(|ch: char| ch.is_ascii_digit()) || rest.contains(|ch: char| ch.is_ascii_digit() || ch == DECIMAL_POINT || SIGNS.contains(&ch)) {
        return None;
    }
    number.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// The seconds a length of time typed as `minutes` and `seconds` on either side of a colon comes
/// to, or `None` when it is not one: the minutes are a sign or none and then digits alone, and the
/// seconds a number with no sign of its own.
fn clock_seconds(minutes: &str, seconds: &str) -> Option<f64> {
    let whole = minutes.strip_prefix(SIGNS).unwrap_or(minutes);
    if whole.is_empty() || !whole.chars().all(|ch| ch.is_ascii_digit()) || seconds.starts_with(SIGNS) {
        return None;
    }
    let length = whole.parse::<f64>().ok()? * SECONDS_PER_MINUTE as f64 + typed_number(seconds)?;
    Some(if minutes.starts_with(MINUS) { -length } else { length }).filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERCENT: NumberSpec = NumberSpec::whole(0, 100, "%");
    const TENTHS: NumberSpec = NumberSpec::fine(0, 1000, 10, 1, "%");
    const OFFSET: NumberSpec = NumberSpec::whole(-200, 200, " MS").with_sign();
    const SPEED: NumberSpec = NumberSpec::fine(1, 2000, 100, 2, "");
    const CLOCK: NumberSpec = NumberSpec::fine(0, 6000, 10, 1, "").as_clock();
    const WHOLE_CLOCK: NumberSpec = NumberSpec::whole(-90, 900, "").as_clock().with_sign();

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
        for spec in [PERCENT, TENTHS, OFFSET, SPEED, CLOCK, WHOLE_CLOCK] {
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

    /// A length of time is written as minutes and seconds, to the places the row is shown to, and
    /// the seconds never read as sixty.
    #[test]
    fn a_length_of_time_is_written_as_minutes_and_seconds() {
        assert_eq!(CLOCK.text(0), "0:00.0");
        assert_eq!(CLOCK.text(5), "0:00.5");
        assert_eq!(CLOCK.text(599), "0:59.9");
        assert_eq!(CLOCK.text(600), "1:00.0");
        assert_eq!(CLOCK.plain(834), "1:23.4");
        assert_eq!(CLOCK.figure(6000), "10:00.0");
        assert_eq!(WHOLE_CLOCK.text(75), "+1:15");
        assert_eq!(WHOLE_CLOCK.text(0), "+0:00");
        assert_eq!(WHOLE_CLOCK.text(-61), "-1:01");
        assert_eq!(WHOLE_CLOCK.plain(75), "1:15", "what is typed over carries no sign of its own");
    }

    /// A length of time is typed as it is written, or as seconds alone. Seconds past a minute are
    /// counted on into the next one.
    #[test]
    fn a_length_of_time_is_typed_as_minutes_and_seconds_or_as_seconds() {
        assert_eq!(CLOCK.parse("1:23.4"), Some(834));
        assert_eq!(CLOCK.parse(" 1:23.4 "), Some(834));
        assert_eq!(CLOCK.parse("83.4"), Some(834));
        assert_eq!(CLOCK.parse("83.44"), Some(834), "rounded to the nearest step");
        assert_eq!(CLOCK.parse("83.46"), Some(835));
        assert_eq!(CLOCK.parse("0:05"), Some(50));
        assert_eq!(CLOCK.parse("2:5"), Some(1250));
        assert_eq!(CLOCK.parse("1:75"), Some(1350), "seconds past a minute are counted on");
        assert_eq!(CLOCK.parse("1:30s"), Some(900), "a unit after the seconds is read past");
        assert_eq!(CLOCK.parse("99:00"), Some(6000), "past the upper end");
        assert_eq!(CLOCK.parse("-0:05"), Some(0), "past the lower end");
        assert_eq!(WHOLE_CLOCK.parse("-1:01"), Some(-61));
        assert_eq!(WHOLE_CLOCK.parse("+1:01"), Some(61));
        for typed in ["", ":", "1:", ":30", "1:2:3", "1:-5", "1:+5", "a:30", "1.5:30", "1:3a0", "1:.", "-:30", "1 :30", "1::30"] {
            assert_eq!(CLOCK.parse(typed), None, "{typed:?} was read as a length of time");
        }
        assert_eq!(PERCENT.parse("1:30"), None, "a row that is no length of time read one");
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
