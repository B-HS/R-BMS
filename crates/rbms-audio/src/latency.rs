//! What an output device itself holds a frame back for after the stream has handed it over.
//!
//! cpal's playback timestamp covers the stream's own buffer and nothing past it, so a device that
//! adds a delay of its own -- a Bluetooth headset adds well over a hundred milliseconds -- is heard
//! that much later than the clock says. The device reports that delay in frames; this module reads
//! it where there is a safe way to (CoreAudio on macOS) and reads nothing anywhere else.
//!
//! Reading goes through the audio host thread ([`crate::host`]) and never through the callback.

use std::time::Duration;

/// The delay an output device adds before a frame it has been handed is heard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutputLatency {
    /// Frames the device reports: its own latency plus its safety offset.
    pub frames: u32,
    /// Those frames as time, at the rate the device runs at.
    pub delay: Duration,
    /// Whether the device is connected over Bluetooth (classic or LE).
    pub bluetooth: bool,
}

impl OutputLatency {
    /// The delay `frames` make at `sample_rate`, and nothing at a rate of zero.
    pub fn of_frames(frames: u32, sample_rate: u32, bluetooth: bool) -> OutputLatency {
        let delay = match sample_rate {
            0 => Duration::ZERO,
            rate => Duration::from_nanos(u64::from(frames) * NANOS_PER_SECOND / u64::from(rate)),
        };
        OutputLatency { frames, delay, bluetooth }
    }

    /// The delay `frames` of a device make: counted at the rate the device said it runs at, which
    /// is what its frames are, or at the stream's rate when the device did not say one that can be
    /// counted at.
    ///
    /// The two differ once something else moves the device off the rate the stream was opened at
    /// (another app, or a Bluetooth headset dropping to its call profile), or when the stream
    /// follows the system default onto a device running at another rate.
    pub fn of_device_frames(frames: u32, device_rate: Option<f64>, stream_rate: u32, bluetooth: bool) -> OutputLatency {
        let rate = device_rate.filter(|rate| (MIN_DEVICE_RATE..=f64::from(u32::MAX)).contains(rate)).map_or(stream_rate, |rate| rate.round() as u32);
        OutputLatency::of_frames(frames, rate, bluetooth)
    }
}

/// Nanoseconds in one second, for turning frames into time.
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// The lowest device rate frames are counted at: below one frame a second a device is not saying
/// a rate at all.
const MIN_DEVICE_RATE: f64 = 1.0;

/// The device a question about the output delay is put to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LatencyTarget {
    /// Whichever device the system calls its default output at the moment of asking.
    SystemDefault,
    /// The output device of this name.
    Named(String),
}

impl LatencyTarget {
    /// Where to ask about the device a stream plays on.
    ///
    /// A stream opened on the device that was the system default when it opened is opened by cpal
    /// as CoreAudio's default output unit, which moves to the new default when the system default
    /// changes, so it is asked about whatever the default is now. That holds for a stream opened
    /// with no device named, for one that fell back to the default because the named device was
    /// missing, and for one opened by name on the device that happened to be the default. Any other
    /// stream stays on the device it was opened on and is asked about that one by name.
    pub fn of_stream(device_name: &str, opened_on_system_default: bool) -> LatencyTarget {
        match opened_on_system_default {
            true => LatencyTarget::SystemDefault,
            false => LatencyTarget::Named(device_name.to_owned()),
        }
    }
}

/// What the device a question went to answered: its name, and the delay it reported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceLatency {
    /// The name of the device that answered, which for [`LatencyTarget::SystemDefault`] is the
    /// default at the moment of asking rather than the one the stream was opened on.
    pub device_name: String,
    /// The delay it reported.
    pub latency: OutputLatency,
}

/// Read the delay the output device `target` names adds, its frames turned into time at the rate
/// the device runs at, or at `stream_rate` when the device does not say.
///
/// `None` when the device cannot be found or does not answer.
#[cfg(target_os = "macos")]
pub(crate) fn read_output_latency(target: &LatencyTarget, stream_rate: u32) -> Option<DeviceLatency> {
    macos::read(target, stream_rate)
}

/// Nothing to read here: no safe way to ask the device exists off macOS, so the stream's own
/// timestamp is all there is.
#[cfg(not(target_os = "macos"))]
pub(crate) fn read_output_latency(_target: &LatencyTarget, _stream_rate: u32) -> Option<DeviceLatency> {
    None
}

#[cfg(target_os = "macos")]
mod macos {
    use coreaudio::audio_unit::macos_helpers::{
        audio_unit_from_device_id, get_default_device_id, get_device_id_from_name, get_device_name, get_device_transport_type,
    };
    use coreaudio::audio_unit::{Element, Scope};

    use super::{DeviceLatency, LatencyTarget, OutputLatency};

    /// `kAudioDevicePropertyLatency`: frames the device itself adds.
    pub(super) const DEVICE_LATENCY_SELECTOR: u32 = u32::from_be_bytes(*b"ltnc");

    /// `kAudioDevicePropertySafetyOffset`: frames the device keeps between the stream and its own
    /// position.
    pub(super) const SAFETY_OFFSET_SELECTOR: u32 = u32::from_be_bytes(*b"saft");

    /// `kAudioUnitProperty_SampleRate`. Read on the output scope of an output unit's output
    /// element, it is the rate of the device side of the unit: the rate the device's frames are at.
    pub(super) const SAMPLE_RATE_PROPERTY: u32 = 2;

    /// `kAudioDeviceTransportTypeBluetooth`.
    pub(super) const BLUETOOTH_TRANSPORT: u32 = u32::from_be_bytes(*b"blue");

    /// `kAudioDeviceTransportTypeBluetoothLE`.
    pub(super) const BLUETOOTH_LE_TRANSPORT: u32 = u32::from_be_bytes(*b"blea");

    /// Whether a transport type is one of the two Bluetooth ones.
    pub(super) fn is_bluetooth(transport: u32) -> bool {
        matches!(transport, BLUETOOTH_TRANSPORT | BLUETOOTH_LE_TRANSPORT)
    }

    /// Ask the device behind an output unit opened on it, which answers the device's own
    /// properties through the safe property read of `coreaudio-rs`.
    pub(super) fn read(target: &LatencyTarget, stream_rate: u32) -> Option<DeviceLatency> {
        let device = match target {
            LatencyTarget::SystemDefault => get_default_device_id(false)?,
            LatencyTarget::Named(name) => get_device_id_from_name(name, false)?,
        };
        let unit = audio_unit_from_device_id(device, false).ok()?;
        let latency = unit.get_property::<u32>(DEVICE_LATENCY_SELECTOR, Scope::Output, Element::Output).ok()?;
        let safety = unit.get_property::<u32>(SAFETY_OFFSET_SELECTOR, Scope::Output, Element::Output).unwrap_or_default();
        let device_rate = unit.get_property::<f64>(SAMPLE_RATE_PROPERTY, Scope::Output, Element::Output).ok();
        let bluetooth = get_device_transport_type(device).is_ok_and(is_bluetooth);
        let device_name = match (get_device_name(device), target) {
            (Ok(name), _) => name,
            (Err(_), LatencyTarget::Named(name)) => name.clone(),
            (Err(_), LatencyTarget::SystemDefault) => String::new(),
        };
        Some(DeviceLatency { device_name, latency: OutputLatency::of_device_frames(latency.saturating_add(safety), device_rate, stream_rate, bluetooth) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What an AirPods Pro 2 reports on the development machine: 7,680 frames at 48 kHz.
    const AIRPODS_FRAMES: u32 = 7_680;

    #[test]
    fn frames_turn_into_time_at_the_stream_rate() {
        let airpods = OutputLatency::of_frames(AIRPODS_FRAMES, 48_000, true);
        assert_eq!(airpods.delay, Duration::from_millis(160));
        assert!(airpods.bluetooth);
        assert_eq!(OutputLatency::of_frames(AIRPODS_FRAMES, 96_000, true).delay, Duration::from_millis(80));
    }

    #[test]
    fn a_stream_without_a_rate_adds_no_delay() {
        assert_eq!(OutputLatency::of_frames(AIRPODS_FRAMES, 0, false).delay, Duration::ZERO);
    }

    /// A device's frames are at the device's rate: a headset that reports 7,680 frames while it
    /// runs at 48 kHz adds 160 ms, whatever rate the stream feeding it was opened at.
    #[test]
    fn a_device_s_frames_are_counted_at_the_rate_the_device_runs_at() {
        assert_eq!(OutputLatency::of_device_frames(AIRPODS_FRAMES, Some(48_000.0), 44_100, true), OutputLatency::of_frames(AIRPODS_FRAMES, 48_000, true));
        assert_eq!(OutputLatency::of_device_frames(AIRPODS_FRAMES, Some(16_000.0), 48_000, true).delay, Duration::from_millis(480));
        assert_eq!(OutputLatency::of_device_frames(AIRPODS_FRAMES, Some(47_999.6), 44_100, true).delay, Duration::from_millis(160));
    }

    #[test]
    fn a_device_that_gives_no_rate_to_count_at_is_counted_at_the_stream_rate() {
        let at_stream_rate = OutputLatency::of_frames(AIRPODS_FRAMES, 44_100, true);
        for unusable in [None, Some(0.0), Some(0.5), Some(-48_000.0), Some(f64::NAN), Some(f64::INFINITY), Some(1e12)] {
            assert_eq!(OutputLatency::of_device_frames(AIRPODS_FRAMES, unusable, 44_100, true), at_stream_rate, "{unusable:?}");
        }
    }

    /// The two ways a stream comes to be on the system default -- nothing named, or the named
    /// device missing and the default taken instead -- and a named device that was the default
    /// when the stream opened all follow the default, so the question goes to whatever the default
    /// is now. Only a stream that stayed on another named device is asked about it by name.
    #[test]
    fn a_stream_on_the_system_default_asks_the_default_of_now_and_any_other_asks_its_device() {
        assert_eq!(LatencyTarget::of_stream("MacBook Pro Speakers", true), LatencyTarget::SystemDefault);
        assert_eq!(LatencyTarget::of_stream("USB DAC", false), LatencyTarget::Named("USB DAC".to_owned()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_selectors_are_the_four_character_codes_core_audio_declares() {
        assert_eq!(macos::DEVICE_LATENCY_SELECTOR, 0x6c74_6e63);
        assert_eq!(macos::SAFETY_OFFSET_SELECTOR, 0x7361_6674);
        assert_eq!(macos::BLUETOOTH_TRANSPORT, 0x626c_7565);
        assert_eq!(macos::BLUETOOTH_LE_TRANSPORT, 0x626c_6561);
        assert_eq!(macos::SAMPLE_RATE_PROPERTY, 2);
        assert!(macos::is_bluetooth(macos::BLUETOOTH_LE_TRANSPORT));
        assert!(!macos::is_bluetooth(u32::from_be_bytes(*b"bltn")));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn nothing_is_read_off_macos() {
        assert_eq!(read_output_latency(&LatencyTarget::SystemDefault, 48_000), None);
        assert_eq!(read_output_latency(&LatencyTarget::Named("any device".to_owned()), 48_000), None);
    }

    /// Prints what every output device on this machine reports, and what the system default
    /// does, through the host thread the app asks from. A machine without devices prints nothing
    /// and passes.
    #[test]
    fn every_output_device_answers_or_is_skipped() {
        let named = crate::host::output_device_names().into_iter().map(LatencyTarget::Named);
        for target in std::iter::once(LatencyTarget::SystemDefault).chain(named) {
            match crate::host::output_device_latency(&target, 44_100) {
                Some(Some(DeviceLatency { device_name, latency })) => println!(
                    "output latency: {target:?} -> {device_name}: {} frames, {:?} at the device's rate (stream at 44.1 kHz), bluetooth {}",
                    latency.frames, latency.delay, latency.bluetooth
                ),
                Some(None) => println!("output latency: {target:?}: not reported"),
                None => println!("output latency: {target:?}: no answer in time"),
            }
        }
    }
}
