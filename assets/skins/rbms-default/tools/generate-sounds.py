# /// script
# requires-python = ">=3.11"
# ///
from __future__ import annotations

import math
import struct
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOUND = ROOT / 'sound'

SAMPLE_RATE = 44100
SAMPLE_WIDTH = 2
CHANNEL_COUNT = 1
PEAK_LEVEL = 32767
TAU = math.pi * 2
REFERENCE_PITCH = 440.0
SEMITONE_RATIO = 2.0 ** (1.0 / 12.0)
ATTACK_SECONDS = 0.005
RELEASE_RATIO = 0.55
EDGE_FADE_SAMPLES = 96
NOISE_MULTIPLIER = 1103515245
NOISE_INCREMENT = 12345
NOISE_MASK = 0x7FFFFFFF
NOISE_SCALE = 0x3FFFFFFF


def pitch(semitones: float) -> float:
    return REFERENCE_PITCH * SEMITONE_RATIO**semitones


def sine_wave(phase: float) -> float:
    return math.sin(phase)


def triangle_wave(phase: float) -> float:
    position = (phase / TAU) % 1.0
    return 4.0 * abs(position - 0.5) - 1.0


def segment(
    start: float,
    length: float,
    frequency_start: float,
    frequency_end: float,
    amplitude: float,
    shape=sine_wave,
    noise: float = 0.0,
) -> tuple[float, float, float, float, float, object, float]:
    return start, length, frequency_start, frequency_end, amplitude, shape, noise


def envelope(index: int, count: int) -> float:
    attack = max(1, int(ATTACK_SECONDS * SAMPLE_RATE))
    release = max(1, int(count * RELEASE_RATIO))
    if index < attack:
        return index / attack
    remaining = count - index
    if remaining < release:
        return remaining / release
    return 1.0


def synthesise(duration: float, segments: tuple) -> list[float]:
    total = int(duration * SAMPLE_RATE)
    buffer = [0.0] * total
    for start, length, frequency_start, frequency_end, amplitude, shape, noise in segments:
        begin = int(start * SAMPLE_RATE)
        count = max(1, int(length * SAMPLE_RATE))
        phase = 0.0
        state = (begin * NOISE_MULTIPLIER + NOISE_INCREMENT) & NOISE_MASK
        for index in range(count):
            position = begin + index
            if position >= total:
                break
            ratio = index / max(count - 1, 1)
            frequency = frequency_start + (frequency_end - frequency_start) * ratio
            phase += TAU * frequency / SAMPLE_RATE
            state = (state * NOISE_MULTIPLIER + NOISE_INCREMENT) & NOISE_MASK
            grain = state / NOISE_SCALE - 1.0
            value = shape(phase) * (1.0 - noise) + grain * noise
            buffer[position] += value * amplitude * envelope(index, count)
    for index in range(min(EDGE_FADE_SAMPLES, total)):
        ramp = index / EDGE_FADE_SAMPLES
        buffer[index] *= ramp
        buffer[total - 1 - index] *= ramp
    return buffer


def write_stem(path: Path, buffer: list[float]) -> None:
    frames = bytearray()
    for value in buffer:
        clamped = max(-1.0, min(1.0, value))
        frames += struct.pack('<h', int(clamped * PEAK_LEVEL))
    with wave.open(str(path), 'wb') as handle:
        handle.setnchannels(CHANNEL_COUNT)
        handle.setsampwidth(SAMPLE_WIDTH)
        handle.setframerate(SAMPLE_RATE)
        handle.writeframes(bytes(frames))


def arpeggio(semitones: tuple[float, ...], step: float, length: float, amplitude: float, shape=sine_wave) -> tuple:
    return tuple(segment(index * step, length, pitch(value), pitch(value), amplitude, shape) for index, value in enumerate(semitones))


def stems() -> dict[str, tuple[float, tuple]]:
    return {
        'scratch': (0.18, (segment(0.0, 0.17, 720.0, 180.0, 0.55, triangle_wave, 0.86),)),
        'f-open': (
            0.25,
            (
                segment(0.0, 0.24, 320.0, 960.0, 0.40),
                segment(0.02, 0.20, 640.0, 1920.0, 0.13, triangle_wave),
            ),
        ),
        'f-close': (
            0.25,
            (
                segment(0.0, 0.24, 960.0, 320.0, 0.40),
                segment(0.02, 0.20, 1920.0, 640.0, 0.13, triangle_wave),
            ),
        ),
        'o-change': (0.06, (segment(0.0, 0.055, 1320.0, 980.0, 0.42, triangle_wave, 0.22),)),
        'o-open': (0.20, (segment(0.0, 0.19, 440.0, 880.0, 0.38), segment(0.0, 0.19, 880.0, 1760.0, 0.10, triangle_wave))),
        'o-close': (0.20, (segment(0.0, 0.19, 880.0, 440.0, 0.38), segment(0.0, 0.19, 1760.0, 880.0, 0.10, triangle_wave))),
        'playready': (
            0.35,
            (
                segment(0.0, 0.14, pitch(3), pitch(3), 0.42),
                segment(0.16, 0.18, pitch(10), pitch(10), 0.42),
            ),
        ),
        'playstop': (
            0.30,
            (
                segment(0.0, 0.13, pitch(10), pitch(10), 0.40),
                segment(0.14, 0.15, pitch(-2), pitch(-2), 0.40, triangle_wave),
            ),
        ),
        'clear': (0.60, arpeggio((3, 7, 10, 15), 0.13, 0.20, 0.34)),
        'fail': (0.55, arpeggio((-2, -5, -9), 0.16, 0.22, 0.36, triangle_wave)),
        'resultclose': (
            0.30,
            (
                segment(0.0, 0.13, pitch(7), pitch(7), 0.38),
                segment(0.14, 0.15, pitch(0), pitch(0), 0.38),
            ),
        ),
        'course_clear': (0.60, arpeggio((3, 10, 15, 19), 0.12, 0.20, 0.32)),
        'course_fail': (0.55, arpeggio((-4, -9, -14), 0.16, 0.22, 0.34, triangle_wave)),
        'course_close': (
            0.30,
            (
                segment(0.0, 0.13, pitch(10), pitch(10), 0.36),
                segment(0.14, 0.15, pitch(3), pitch(3), 0.36),
            ),
        ),
        'guide-pg': (0.09, (segment(0.0, 0.085, pitch(15), pitch(15), 0.40),)),
        'guide-gr': (0.09, (segment(0.0, 0.085, pitch(12), pitch(12), 0.40),)),
        'guide-gd': (0.09, (segment(0.0, 0.085, pitch(8), pitch(8), 0.40),)),
        'guide-bd': (0.09, (segment(0.0, 0.085, pitch(3), pitch(3), 0.40, triangle_wave),)),
        'guide-pr': (0.09, (segment(0.0, 0.085, pitch(-2), pitch(-2), 0.40, triangle_wave),)),
        'guide-ms': (0.09, (segment(0.0, 0.085, pitch(-7), pitch(-7), 0.42, triangle_wave),)),
        'select': (0.50, arpeggio((5, 12, 17), 0.11, 0.18, 0.34)),
        'decide': (
            0.60,
            arpeggio((5, 12, 17, 21), 0.11, 0.20, 0.30) + (segment(0.0, 0.58, pitch(-7), pitch(-7), 0.16, triangle_wave),),
        ),
    }


def main() -> None:
    SOUND.mkdir(parents=True, exist_ok=True)
    for name, (duration, segments) in stems().items():
        write_stem(SOUND / f'{name}.wav', synthesise(duration, segments))


if __name__ == '__main__':
    main()
