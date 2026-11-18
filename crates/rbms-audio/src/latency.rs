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
    /// Those frames as time, at the rate the stream runs at.
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
}

/// Nanoseconds in one second, for turning frames into time.
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Read the delay the output device named `device_name` adds, its frames turned into time at
/// `stream_rate` (the rate the stream was opened at, which the backend set the device to).
///
/// `None` when the device cannot be found or does not answer.
#[cfg(target_os = "macos")]
pub(crate) fn read_output_latency(device_name: &str, stream_rate: u32) -> Option<OutputLatency> {
    macos::read(device_name, stream_rate)
}

/// Nothing to read here: no safe way to ask the device exists off macOS, so the stream's own
/// timestamp is all there is.
#[cfg(not(target_os = "macos"))]
pub(crate) fn read_output_latency(_device_name: &str, _stream_rate: u32) -> Option<OutputLatency> {
    None
}

#[cfg(target_os = "macos")]
mod macos {
    use coreaudio::audio_unit::macos_helpers::{audio_unit_from_device_id, get_device_id_from_name, get_device_transport_type};
    use coreaudio::audio_unit::{Element, Scope};

    use super::OutputLatency;

    /// `kAudioDevicePropertyLatency`: frames the device itself adds.
    pub(super) const DEVICE_LATENCY_SELECTOR: u32 = u32::from_be_bytes(*b"ltnc");

    /// `kAudioDevicePropertySafetyOffset`: frames the device keeps between the stream and its own
    /// position.
    pub(super) const SAFETY_OFFSET_SELECTOR: u32 = u32::from_be_bytes(*b"saft");

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
    pub(super) fn read(device_name: &str, stream_rate: u32) -> Option<OutputLatency> {
        let device = get_device_id_from_name(device_name, false)?;
        let unit = audio_unit_from_device_id(device, false).ok()?;
        let latency = unit.get_property::<u32>(DEVICE_LATENCY_SELECTOR, Scope::Output, Element::Output).ok()?;
        let safety = unit.get_property::<u32>(SAFETY_OFFSET_SELECTOR, Scope::Output, Element::Output).unwrap_or_default();
        let bluetooth = get_device_transport_type(device).is_ok_and(is_bluetooth);
        Some(OutputLatency::of_frames(latency.saturating_add(safety), stream_rate, bluetooth))
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

    #[cfg(target_os = "macos")]
    #[test]
    fn the_selectors_are_the_four_character_codes_core_audio_declares() {
        assert_eq!(macos::DEVICE_LATENCY_SELECTOR, 0x6c74_6e63);
        assert_eq!(macos::SAFETY_OFFSET_SELECTOR, 0x7361_6674);
        assert_eq!(macos::BLUETOOTH_TRANSPORT, 0x626c_7565);
        assert_eq!(macos::BLUETOOTH_LE_TRANSPORT, 0x626c_6561);
        assert!(macos::is_bluetooth(macos::BLUETOOTH_LE_TRANSPORT));
        assert!(!macos::is_bluetooth(u32::from_be_bytes(*b"bltn")));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn nothing_is_read_off_macos() {
        assert_eq!(read_output_latency("any device", 48_000), None);
    }

    /// Prints what every output device on this machine reports, through the host thread the app
    /// asks from. A machine without devices prints nothing and passes.
    #[test]
    fn every_output_device_answers_or_is_skipped() {
        for name in crate::host::output_device_names() {
            match crate::host::output_device_latency(&name, 48_000) {
                Some(latency) => println!("output latency: {name}: {} frames, {:?} at 48 kHz, bluetooth {}", latency.frames, latency.delay, latency.bluetooth),
                None => println!("output latency: {name}: not reported"),
            }
        }
    }
}
