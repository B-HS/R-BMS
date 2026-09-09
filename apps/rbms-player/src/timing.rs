//! Timing instrumentation for the audio clock: a ring of per-input samples (interpolated clock vs
//! the quantised one, plus the judgement error), the summary statistics the debug overlay shows,
//! and the periodic soak-log row that records stability across a long unattended run.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How many recent input events the probe keeps for statistics and the per-sample CSV dump.
pub(crate) const TIMING_SAMPLES: usize = 4096;

/// Column header of the per-sample CSV written by [`TimingProbe::write_csv`].
pub(crate) const TIMING_CSV_HEADER: &str = "wall_us,input_at_us,quantized_us,interp_minus_quantized_us,judge_delta_us,frames_at_start,buffer_frames";

/// Environment variable holding the soak-log path. Unset means the soak log is off.
pub(crate) const SOAK_LOG_ENV: &str = "RBMS_SOAK_LOG";

/// Environment variable holding the path the per-sample timing CSV is dumped to on result entry.
pub(crate) const TIMING_CSV_ENV: &str = "RBMS_TIMING_CSV";

/// How often a soak row is appended.
pub(crate) const SOAK_LOG_INTERVAL: Duration = Duration::from_secs(60);

/// How many recent frame durations back the soak row's fps average and p95 frame time.
pub(crate) const SOAK_FRAME_SAMPLES: usize = 2048;

/// Column header of the soak log, written once when the file is created.
pub(crate) const SOAK_CSV_HEADER: &str = "ts_iso,stage,uptime_s,rss_mb,fps_avg,frame_ms_p95,active_voices,underruns,drops,steals,hard_steals,late,ts_fallbacks,retire_overflows,interp_sd_us,judge_sd_us";

/// Rank of the reported judgement-error and frame-time percentiles.
const REPORTED_PERCENTILE: f64 = 0.95;

/// Readings needed before the clock fit reports anything. A line through two points has no
/// residual, so the figure would read a misleading zero.
const MIN_FIT_SAMPLES: usize = 3;

const MICROSECONDS_PER_MILLISECOND: f64 = 1_000.0;
const MILLISECONDS_PER_SECOND: f32 = 1_000.0;
const SECONDS_PER_MINUTE: i64 = 60;
const MINUTES_PER_HOUR: i64 = 60;
const HOURS_PER_DAY: i64 = 24;
const SECONDS_PER_HOUR: i64 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;
const SECONDS_PER_DAY: i64 = SECONDS_PER_HOUR * HOURS_PER_DAY;

/// Civil-calendar constants of the shift-to-March era algorithm used by [`iso_utc`].
const DAYS_PER_ERA: i64 = 146_097;
const DAYS_PER_ERA_LAST_INDEX: i64 = DAYS_PER_ERA - 1;
const YEARS_PER_ERA: i64 = 400;
const YEARS_PER_LEAP_CYCLE: i64 = 4;
const YEARS_PER_CENTURY: i64 = 100;
const DAYS_PER_FOUR_COMMON_YEARS: i64 = 1_460;
const DAYS_PER_CENTURY: i64 = 36_524;
const DAYS_PER_COMMON_YEAR: i64 = 365;
const UNIX_EPOCH_DAYS_AFTER_ERA_START: i64 = 719_468;
const MONTH_STEP_NUMERATOR: i64 = 5;
const MONTH_STEP_OFFSET: i64 = 2;
const MONTH_STEP_DENOMINATOR: i64 = 153;
const MONTHS_BEFORE_MARCH_WRAP: i64 = 10;
const MARCH_MONTH_NUMBER: i64 = 3;
const MONTHS_PER_YEAR_AFTER_WRAP: i64 = 9;
const LAST_MONTH_OF_SHIFTED_YEAR: i64 = 2;

/// One input event measured on both clocks, stamped with the wall clock it was read at.
/// `judge_delta_us` is present only when the press produced a judgement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TimingSample {
    pub wall_us: i64,
    pub input_at_us: i64,
    pub quantized_us: i64,
    pub judge_delta_us: Option<i64>,
    pub frames_at_start: u64,
    pub buffer_frames: u32,
}

/// Summary of the samples currently in the ring. All values are microseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct TimingStats {
    pub n: usize,
    pub clock_n: usize,
    /// How far the interpolated clock strays from a straight line in wall-clock time. This is the
    /// figure that says whether the clock is genuinely interpolated: a clock that only steps once
    /// per callback scatters around the fitted line by `buffer_period / sqrt(12)` (3.08 ms for 512
    /// frames at 48 kHz), while an interpolated one stays inside its own read jitter. Comparing the
    /// interpolated reading against the quantised one cannot show this, because the two are read
    /// from the same record and differ by exactly one buffer whatever the interpolation does.
    pub interp_residual_sd_us: f64,
    pub interp_minus_quantized_mean_us: f64,
    pub interp_minus_quantized_max_us: i64,
    pub judge_delta_mean_us: f64,
    pub judge_delta_sd_us: f64,
    pub judge_delta_p95_abs_us: i64,
}

/// Online least-squares fit of the interpolated clock against the wall clock, kept with Welford
/// updates so a whole soak run can be summarised without storing it. Only the scatter around the
/// fitted line is reported: the slope absorbs the constant rate difference between the device clock
/// and the system clock, which is real and not an error.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ClockFit {
    n: usize,
    mean_x: f64,
    mean_y: f64,
    m2_x: f64,
    m2_y: f64,
    c_xy: f64,
}

impl ClockFit {
    pub(crate) fn push(&mut self, wall_us: i64, clock_us: i64) {
        let (x, y) = (wall_us as f64, clock_us as f64);
        self.n += 1;
        let dx = x - self.mean_x;
        let dy = y - self.mean_y;
        self.mean_x += dx / self.n as f64;
        self.mean_y += dy / self.n as f64;
        self.m2_x += dx * (x - self.mean_x);
        self.m2_y += dy * (y - self.mean_y);
        self.c_xy += dx * (y - self.mean_y);
    }

    pub(crate) fn len(&self) -> usize {
        self.n
    }

    /// Standard deviation of the residuals around the fitted line, in µs. Zero until there are
    /// enough readings, or when the readings span no wall-clock time at all.
    pub(crate) fn residual_sd_us(&self) -> f64 {
        if self.n < MIN_FIT_SAMPLES || self.m2_x <= 0.0 {
            return 0.0;
        }
        let slope = self.c_xy / self.m2_x;
        let residual = self.m2_y - slope * self.c_xy;
        (residual.max(0.0) / self.n as f64).sqrt()
    }

    pub(crate) fn clear(&mut self) {
        *self = ClockFit::default();
    }
}

/// Fixed-capacity ring of the most recent [`TimingSample`]s, oldest first once it wraps, plus the
/// running clock fit, which spans the whole run rather than the ring.
pub(crate) struct TimingProbe {
    samples: Vec<TimingSample>,
    next: usize,
    wrapped: bool,
    fit: ClockFit,
}

impl Default for TimingProbe {
    fn default() -> Self {
        TimingProbe { samples: Vec::with_capacity(TIMING_SAMPLES), next: 0, wrapped: false, fit: ClockFit::default() }
    }
}

impl TimingProbe {
    /// Record one reading of the interpolated clock against the wall clock. Called every frame the
    /// song clock is live, so the fit has samples even in an unattended autoplay soak where nothing
    /// is ever pressed.
    pub(crate) fn push_clock(&mut self, wall_us: i64, clock_us: i64) {
        self.fit.push(wall_us, clock_us);
    }

    pub(crate) fn push(&mut self, sample: TimingSample) {
        if self.samples.len() < TIMING_SAMPLES {
            self.samples.push(sample);
            self.next = self.samples.len() % TIMING_SAMPLES;
            self.wrapped = self.next == 0;
            return;
        }
        self.samples[self.next] = sample;
        self.next = (self.next + 1) % TIMING_SAMPLES;
        self.wrapped = true;
    }

    pub(crate) fn len(&self) -> usize {
        self.samples.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Every retained sample, oldest first.
    pub(crate) fn ordered(&self) -> Vec<TimingSample> {
        if !self.wrapped {
            return self.samples.clone();
        }
        let (head, tail) = self.samples.split_at(self.next);
        tail.iter().chain(head.iter()).copied().collect()
    }

    pub(crate) fn clear(&mut self) {
        self.samples.clear();
        self.next = 0;
        self.wrapped = false;
        self.fit.clear();
    }

    pub(crate) fn stats(&self) -> TimingStats {
        let gaps: Vec<f64> = self.samples.iter().map(|s| (s.input_at_us - s.quantized_us) as f64).collect();
        let deltas: Vec<f64> = self.samples.iter().filter_map(|s| s.judge_delta_us).map(|d| d as f64).collect();
        let mut abs_deltas: Vec<i64> = self.samples.iter().filter_map(|s| s.judge_delta_us).map(i64::abs).collect();
        abs_deltas.sort_unstable();
        TimingStats {
            n: self.samples.len(),
            clock_n: self.fit.len(),
            interp_residual_sd_us: self.fit.residual_sd_us(),
            interp_minus_quantized_mean_us: mean(&gaps),
            interp_minus_quantized_max_us: self.samples.iter().map(|s| (s.input_at_us - s.quantized_us).abs()).max().unwrap_or(0),
            judge_delta_mean_us: mean(&deltas),
            judge_delta_sd_us: standard_deviation(&deltas),
            judge_delta_p95_abs_us: percentile_i64(&abs_deltas, REPORTED_PERCENTILE),
        }
    }

    /// The ring as CSV text, header first, oldest sample first. An absent judgement error is
    /// written as an empty field.
    pub(crate) fn csv(&self) -> String {
        let mut out = String::from(TIMING_CSV_HEADER);
        out.push('\n');
        for s in self.ordered() {
            let judge = s.judge_delta_us.map(|d| d.to_string()).unwrap_or_default();
            out.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                s.wall_us,
                s.input_at_us,
                s.quantized_us,
                s.input_at_us - s.quantized_us,
                judge,
                s.frames_at_start,
                s.buffer_frames
            ));
        }
        out
    }

    pub(crate) fn write_csv(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, self.csv())
    }
}

/// The per-interval values a soak row records. Everything the audio engine and mixer report is
/// cumulative; the two standard deviations come from [`TimingProbe::stats`].
pub(crate) struct SoakSnapshot {
    pub stage: &'static str,
    pub rss_mb: f32,
    pub active_voices: u32,
    pub underruns: u64,
    pub drops: u64,
    pub steals: u64,
    pub hard_steals: u64,
    pub late: u64,
    pub ts_fallbacks: u64,
    pub retire_overflows: u64,
    pub interp_sd_us: f64,
    pub judge_sd_us: f64,
}

/// One soak-log line: the 16 columns of [`SOAK_CSV_HEADER`], in that order.
pub(crate) fn soak_csv_row(ts_iso: &str, uptime_s: u64, fps_avg: f32, frame_ms_p95: f32, snapshot: &SoakSnapshot) -> String {
    format!(
        "{},{},{},{:.1},{:.1},{:.2},{},{},{},{},{},{},{},{},{:.2},{:.2}",
        ts_iso,
        snapshot.stage,
        uptime_s,
        snapshot.rss_mb,
        fps_avg,
        frame_ms_p95,
        snapshot.active_voices,
        snapshot.underruns,
        snapshot.drops,
        snapshot.steals,
        snapshot.hard_steals,
        snapshot.late,
        snapshot.ts_fallbacks,
        snapshot.retire_overflows,
        snapshot.interp_sd_us,
        snapshot.judge_sd_us
    )
}

/// Appends one CSV row per [`SOAK_LOG_INTERVAL`] while [`SOAK_LOG_ENV`] names a path. Frame
/// durations are collected every frame so the row can report an honest average and p95.
pub(crate) struct SoakLogger {
    path: Option<PathBuf>,
    started: Instant,
    last_row_at: Instant,
    frame_ms: Vec<f32>,
    next_frame: usize,
    header_written: bool,
}

impl SoakLogger {
    /// Reads [`SOAK_LOG_ENV`]; a missing or blank value leaves the logger inert.
    pub(crate) fn from_env() -> Self {
        Self::from_path_value(std::env::var(SOAK_LOG_ENV).ok())
    }

    fn from_path_value(raw: Option<String>) -> Self {
        let now = Instant::now();
        SoakLogger {
            path: env_path(raw),
            started: now,
            last_row_at: now,
            frame_ms: Vec::with_capacity(SOAK_FRAME_SAMPLES),
            next_frame: 0,
            header_written: false,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.path.is_some()
    }

    pub(crate) fn record_frame(&mut self, dt_seconds: f32) {
        if self.path.is_none() || !dt_seconds.is_finite() || dt_seconds <= 0.0 {
            return;
        }
        let ms = dt_seconds * MILLISECONDS_PER_SECOND;
        if self.frame_ms.len() < SOAK_FRAME_SAMPLES {
            self.frame_ms.push(ms);
            return;
        }
        self.frame_ms[self.next_frame] = ms;
        self.next_frame = (self.next_frame + 1) % SOAK_FRAME_SAMPLES;
    }

    pub(crate) fn due(&self, now: Instant) -> bool {
        self.path.is_some() && now.duration_since(self.last_row_at) >= SOAK_LOG_INTERVAL
    }

    pub(crate) fn fps_avg(&self) -> f32 {
        let mean_ms = mean(&self.frame_ms.iter().map(|v| *v as f64).collect::<Vec<f64>>()) as f32;
        if mean_ms > 0.0 { MILLISECONDS_PER_SECOND / mean_ms } else { 0.0 }
    }

    pub(crate) fn frame_ms_p95(&self) -> f32 {
        let mut sorted = self.frame_ms.clone();
        sorted.sort_by(f32::total_cmp);
        percentile_f32(&sorted, REPORTED_PERCENTILE)
    }

    /// Appends one row and rearms the interval. Errors are returned so the caller can report the
    /// failure once instead of every interval.
    pub(crate) fn write_row(&mut self, now: Instant, snapshot: &SoakSnapshot) -> std::io::Result<()> {
        let Some(path) = self.path.clone() else {
            return Ok(());
        };
        self.last_row_at = now;
        let uptime = now.duration_since(self.started).as_secs();
        let row = soak_csv_row(&iso_utc_now(), uptime, self.fps_avg(), self.frame_ms_p95(), snapshot);
        let write_header = !self.header_written && !path.exists();
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
        if write_header {
            writeln!(file, "{SOAK_CSV_HEADER}")?;
        }
        self.header_written = true;
        writeln!(file, "{row}")
    }
}

/// A configured file path from an environment variable: blank or unset means the feature is off.
pub(crate) fn env_path(raw: Option<String>) -> Option<PathBuf> {
    raw.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Population standard deviation; zero for fewer than two values.
fn standard_deviation(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    (values.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / values.len() as f64).sqrt()
}

/// Nearest-rank percentile of an already sorted slice.
fn percentile_index(len: usize, rank: f64) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some((((len as f64) * rank).ceil() as usize).clamp(1, len) - 1)
}

fn percentile_i64(sorted: &[i64], rank: f64) -> i64 {
    percentile_index(sorted.len(), rank).map(|i| sorted[i]).unwrap_or(0)
}

fn percentile_f32(sorted: &[f32], rank: f64) -> f32 {
    percentile_index(sorted.len(), rank).map(|i| sorted[i]).unwrap_or(0.0)
}

/// Microseconds rendered as milliseconds, the unit the overlay reports timing in.
pub(crate) fn us_to_millis(us: f64) -> f64 {
    us / MICROSECONDS_PER_MILLISECOND
}

fn iso_utc_now() -> String {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    iso_utc(secs)
}

/// RFC 3339 UTC rendering of a Unix timestamp, e.g. `2026-09-09T12:00:00Z`. Uses the shift-to-March
/// era algorithm so no calendar dependency is needed.
pub(crate) fn iso_utc(unix_seconds: i64) -> String {
    let days = unix_seconds.div_euclid(SECONDS_PER_DAY);
    let time_of_day = unix_seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / SECONDS_PER_HOUR;
    let minute = (time_of_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;
    let second = time_of_day % SECONDS_PER_MINUTE;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let shifted = days_since_epoch + UNIX_EPOCH_DAYS_AFTER_ERA_START;
    let era = if shifted >= 0 { shifted } else { shifted - DAYS_PER_ERA_LAST_INDEX } / DAYS_PER_ERA;
    let day_of_era = shifted - era * DAYS_PER_ERA;
    let year_of_era =
        (day_of_era - day_of_era / DAYS_PER_FOUR_COMMON_YEARS + day_of_era / DAYS_PER_CENTURY - day_of_era / DAYS_PER_ERA_LAST_INDEX) / DAYS_PER_COMMON_YEAR;
    let shifted_year = year_of_era + era * YEARS_PER_ERA;
    let day_of_year = day_of_era - (DAYS_PER_COMMON_YEAR * year_of_era + year_of_era / YEARS_PER_LEAP_CYCLE - year_of_era / YEARS_PER_CENTURY);
    let shifted_month = (MONTH_STEP_NUMERATOR * day_of_year + MONTH_STEP_OFFSET) / MONTH_STEP_DENOMINATOR;
    let day = day_of_year - (MONTH_STEP_DENOMINATOR * shifted_month + MONTH_STEP_OFFSET) / MONTH_STEP_NUMERATOR + 1;
    let month = if shifted_month < MONTHS_BEFORE_MARCH_WRAP { shifted_month + MARCH_MONTH_NUMBER } else { shifted_month - MONTHS_PER_YEAR_AFTER_WRAP };
    let year = if month <= LAST_MONTH_OF_SHIFTED_YEAR { shifted_year + 1 } else { shifted_year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(input: i64, quantized: i64, judge: Option<i64>) -> TimingSample {
        TimingSample { wall_us: input, input_at_us: input, quantized_us: quantized, judge_delta_us: judge, frames_at_start: 0, buffer_frames: 512 }
    }

    #[test]
    fn the_probe_keeps_the_newest_samples_oldest_first_once_it_wraps() {
        let mut probe = TimingProbe::default();
        for i in 0..(TIMING_SAMPLES as i64 + 3) {
            probe.push(sample(i, 0, None));
        }
        let ordered = probe.ordered();
        assert_eq!(probe.len(), TIMING_SAMPLES);
        assert_eq!(ordered.len(), TIMING_SAMPLES);
        assert_eq!(ordered.first().expect("oldest").input_at_us, 3);
        assert_eq!(ordered.last().expect("newest").input_at_us, TIMING_SAMPLES as i64 + 2);
    }

    #[test]
    fn an_unwrapped_probe_reports_the_push_order() {
        let mut probe = TimingProbe::default();
        for i in 0..4 {
            probe.push(sample(i, 0, None));
        }
        assert_eq!(probe.ordered().iter().map(|s| s.input_at_us).collect::<Vec<i64>>(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn clearing_the_probe_drops_every_sample_and_resets_the_wrap() {
        let mut probe = TimingProbe::default();
        for i in 0..(TIMING_SAMPLES as i64 + 1) {
            probe.push(sample(i, 0, None));
        }
        probe.clear();
        assert_eq!(probe.len(), 0);
        assert!(probe.ordered().is_empty());
        probe.push(sample(7, 0, None));
        assert_eq!(probe.ordered().iter().map(|s| s.input_at_us).collect::<Vec<i64>>(), vec![7]);
    }

    #[test]
    fn empty_stats_are_all_zero() {
        let stats = TimingProbe::default().stats();
        assert_eq!(stats, TimingStats::default());
    }

    #[test]
    fn the_quantised_gap_reports_the_interpolated_minus_quantized_difference() {
        let mut probe = TimingProbe::default();
        for residual in [-2_000, 0, 2_000, 4_000] {
            probe.push(sample(residual, 0, None));
        }
        let stats = probe.stats();
        assert_eq!(stats.n, 4);
        assert!((stats.interp_minus_quantized_mean_us - 1_000.0).abs() < 1e-9);
        assert_eq!(stats.interp_minus_quantized_max_us, 4_000);
    }

    /// A clock that only steps once per callback scatters around its own trend line by
    /// `buffer_period / sqrt(12)`; an interpolated one stays on the line. This is the figure the
    /// soak gate reads, so both ends of it are pinned here.
    #[test]
    fn the_clock_fit_separates_an_interpolated_clock_from_a_quantised_one() {
        const BUFFER_US: i64 = 10_667;
        const READINGS: i64 = 4_000;
        const FRAME_US: i64 = 8_000;

        let mut interpolated = TimingProbe::default();
        let mut quantised = TimingProbe::default();
        for i in 0..READINGS {
            let wall = i * FRAME_US;
            interpolated.push_clock(wall, wall);
            quantised.push_clock(wall, wall / BUFFER_US * BUFFER_US);
        }
        let smooth = interpolated.stats();
        let stepped = quantised.stats();
        assert_eq!(smooth.clock_n, READINGS as usize);
        assert!(smooth.interp_residual_sd_us < 1.0, "an exactly linear clock must have no scatter, got {}", smooth.interp_residual_sd_us);

        let quantisation_sd = BUFFER_US as f64 / 12.0f64.sqrt();
        assert!(
            (stepped.interp_residual_sd_us - quantisation_sd).abs() < quantisation_sd * 0.1,
            "a per-callback staircase must scatter by about {quantisation_sd} us, got {}",
            stepped.interp_residual_sd_us
        );
        assert!(stepped.interp_residual_sd_us > 1_000.0, "the soak gate must reject a staircase");
    }

    #[test]
    fn the_clock_fit_absorbs_a_constant_rate_difference_between_the_two_clocks() {
        let mut probe = TimingProbe::default();
        for i in 0..1_000i64 {
            let wall = i * 8_000;
            probe.push_clock(wall, wall + wall / 10_000);
        }
        assert!(probe.stats().interp_residual_sd_us < 1.0, "device-clock drift is a slope, not an error");
    }

    #[test]
    fn the_clock_fit_needs_more_than_two_readings() {
        let mut probe = TimingProbe::default();
        probe.push_clock(0, 0);
        probe.push_clock(1_000, 5_000);
        assert_eq!(probe.stats().interp_residual_sd_us, 0.0);
        assert_eq!(probe.stats().clock_n, 2);
    }

    #[test]
    fn clearing_the_probe_also_clears_the_clock_fit() {
        let mut probe = TimingProbe::default();
        for i in 0..10i64 {
            probe.push_clock(i * 1_000, i * 1_000 + i % 3);
        }
        probe.clear();
        assert_eq!(probe.stats().clock_n, 0);
        assert_eq!(probe.stats().interp_residual_sd_us, 0.0);
    }

    #[test]
    fn judge_statistics_ignore_samples_without_a_judgement() {
        let mut probe = TimingProbe::default();
        probe.push(sample(0, 0, Some(-10)));
        probe.push(sample(0, 0, None));
        probe.push(sample(0, 0, Some(10)));
        probe.push(sample(0, 0, Some(30)));
        let stats = probe.stats();
        assert_eq!(stats.n, 4);
        assert!((stats.judge_delta_mean_us - 10.0).abs() < 1e-9);
        assert!((stats.judge_delta_sd_us - 16.329_931_618_554_52).abs() < 1e-6);
    }

    #[test]
    fn the_reported_percentile_uses_the_nearest_rank() {
        let sorted: Vec<i64> = (1..=100).collect();
        assert_eq!(percentile_i64(&sorted, REPORTED_PERCENTILE), 95);
        assert_eq!(percentile_i64(&[7], REPORTED_PERCENTILE), 7);
        assert_eq!(percentile_i64(&[], REPORTED_PERCENTILE), 0);
    }

    #[test]
    fn the_judgement_percentile_is_taken_on_absolute_errors() {
        let mut probe = TimingProbe::default();
        for delta in [-40_000, 1_000, -2_000, 3_000] {
            probe.push(sample(0, 0, Some(delta)));
        }
        assert_eq!(probe.stats().judge_delta_p95_abs_us, 40_000);
    }

    #[test]
    fn the_timing_csv_starts_with_the_header_and_leaves_a_missing_judgement_blank() {
        let mut probe = TimingProbe::default();
        probe.push(TimingSample {
            wall_us: 9_000,
            input_at_us: 1_500,
            quantized_us: 1_000,
            judge_delta_us: Some(-7),
            frames_at_start: 48_000,
            buffer_frames: 512,
        });
        probe.push(TimingSample {
            wall_us: 10_000,
            input_at_us: 2_500,
            quantized_us: 2_000,
            judge_delta_us: None,
            frames_at_start: 96_000,
            buffer_frames: 512,
        });
        let csv = probe.csv();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], TIMING_CSV_HEADER);
        assert_eq!(lines[1], "9000,1500,1000,500,-7,48000,512");
        assert_eq!(lines[2], "10000,2500,2000,500,,96000,512");
        assert_eq!(lines.len(), 3);
        assert_eq!(TIMING_CSV_HEADER.split(',').count(), lines[1].split(',').count());
    }

    #[test]
    fn the_soak_row_has_the_sixteen_header_columns_in_order() {
        let snapshot = SoakSnapshot {
            stage: "Play",
            rss_mb: 182.44,
            active_voices: 37,
            underruns: 0,
            drops: 0,
            steals: 4,
            hard_steals: 0,
            late: 2,
            ts_fallbacks: 1,
            retire_overflows: 0,
            interp_sd_us: 0.41,
            judge_sd_us: 8.9,
        };
        let row = soak_csv_row("2026-09-09T12:00:00Z", 60, 119.63, 9.14, &snapshot);
        assert_eq!(row, "2026-09-09T12:00:00Z,Play,60,182.4,119.6,9.14,37,0,0,4,0,2,1,0,0.41,8.90");
        assert_eq!(SOAK_CSV_HEADER.split(',').count(), 16);
        assert_eq!(row.split(',').count(), SOAK_CSV_HEADER.split(',').count());
    }

    #[test]
    fn a_blank_or_absent_path_leaves_the_soak_logger_inert() {
        for raw in [None, Some(String::new()), Some("   ".to_string())] {
            let mut logger = SoakLogger::from_path_value(raw);
            assert!(!logger.enabled());
            assert!(!logger.due(Instant::now()));
            logger.record_frame(0.008);
            assert_eq!(logger.fps_avg(), 0.0);
            assert_eq!(logger.frame_ms_p95(), 0.0);
        }
        assert_eq!(env_path(Some("  /tmp/soak.csv  ".to_string())), Some(PathBuf::from("/tmp/soak.csv")));
    }

    #[test]
    fn frame_statistics_come_from_the_recorded_durations() {
        let mut logger = SoakLogger::from_path_value(Some("unused".to_string()));
        for ms in 1..=20 {
            logger.record_frame(ms as f32 / MILLISECONDS_PER_SECOND);
        }
        assert!((logger.frame_ms_p95() - 19.0).abs() < 1e-3);
        assert!((logger.fps_avg() - 95.238_1).abs() < 1e-2);
    }

    #[test]
    fn a_non_positive_or_non_finite_frame_duration_is_not_recorded() {
        let mut logger = SoakLogger::from_path_value(Some("unused".to_string()));
        logger.record_frame(0.0);
        logger.record_frame(-1.0);
        logger.record_frame(f32::NAN);
        assert_eq!(logger.frame_ms_p95(), 0.0);
        assert_eq!(logger.fps_avg(), 0.0);
    }

    #[test]
    fn the_frame_ring_keeps_only_the_newest_durations() {
        let mut logger = SoakLogger::from_path_value(Some("unused".to_string()));
        for _ in 0..SOAK_FRAME_SAMPLES {
            logger.record_frame(0.100);
        }
        for _ in 0..SOAK_FRAME_SAMPLES {
            logger.record_frame(0.010);
        }
        assert!((logger.frame_ms_p95() - 10.0).abs() < 1e-3);
    }

    fn scratch_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rbms-{name}-{}", std::process::id()))
    }

    fn soak_snapshot() -> SoakSnapshot {
        SoakSnapshot {
            stage: "Play",
            rss_mb: 182.4,
            active_voices: 12,
            underruns: 0,
            drops: 0,
            steals: 1,
            hard_steals: 0,
            late: 0,
            ts_fallbacks: 0,
            retire_overflows: 0,
            interp_sd_us: 0.4,
            judge_sd_us: 9.0,
        }
    }

    #[test]
    fn writing_the_timing_csv_creates_the_directory_and_the_file() {
        let dir = scratch_dir("timing-csv");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("timing.csv");
        let mut probe = TimingProbe::default();
        probe.push(sample(1_000, 500, Some(3)));
        probe.write_csv(&path).expect("the csv is written");
        let text = std::fs::read_to_string(&path).expect("the csv is readable");
        assert_eq!(text, probe.csv());
        assert!(text.starts_with(TIMING_CSV_HEADER));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_soak_log_writes_the_header_once_and_then_appends_rows() {
        let dir = scratch_dir("soak-log");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("soak.csv");
        let mut logger = SoakLogger::from_path_value(Some(path.to_string_lossy().into_owned()));
        assert!(logger.enabled());
        logger.write_row(Instant::now(), &soak_snapshot()).expect("the first row is written");
        logger.write_row(Instant::now(), &soak_snapshot()).expect("the second row is appended");
        let text = std::fs::read_to_string(&path).expect("the log is readable");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "one header and two rows");
        assert_eq!(lines[0], SOAK_CSV_HEADER);
        for row in &lines[1..] {
            assert_eq!(row.split(',').count(), SOAK_CSV_HEADER.split(',').count());
            assert_eq!(row.split(',').nth(1), Some("Play"));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writing_a_row_rearms_the_interval() {
        let dir = scratch_dir("soak-interval");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("soak.csv");
        let mut logger = SoakLogger::from_path_value(Some(path.to_string_lossy().into_owned()));
        let now = Instant::now();
        assert!(!logger.due(now), "the first row is not due before the interval elapses");
        assert!(logger.due(now + SOAK_LOG_INTERVAL));
        logger.write_row(now + SOAK_LOG_INTERVAL, &soak_snapshot()).expect("the row is written");
        assert!(!logger.due(now + SOAK_LOG_INTERVAL));
        assert!(logger.due(now + SOAK_LOG_INTERVAL + SOAK_LOG_INTERVAL));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_timestamps_match_known_utc_instants() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(946_684_799), "1999-12-31T23:59:59Z");
        assert_eq!(iso_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso_utc(1_788_955_200), "2026-09-09T12:00:00Z");
        assert_eq!(iso_utc(-1), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn microseconds_convert_to_milliseconds() {
        assert!((us_to_millis(1_500.0) - 1.5).abs() < 1e-9);
        assert!((us_to_millis(-250.0) + 0.25).abs() < 1e-9);
    }

    #[test]
    fn an_empty_probe_reports_empty() {
        let mut probe = TimingProbe::default();
        assert!(probe.is_empty());
        probe.push(sample(0, 0, None));
        assert!(!probe.is_empty());
    }
}
