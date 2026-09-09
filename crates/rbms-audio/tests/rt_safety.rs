//! Real-time safety of the audio callback path. The callback runs on a thread with a hard deadline,
//! so it must not touch the allocator: `Mixer::apply` and `Mixer::mix` are held to zero allocations
//! and zero deallocations here, with a counting global allocator armed only around them.
//!
//! Dropping the last `Arc<SampleData>` frees megabytes of PCM, which is the deallocation this pins.
//! It becomes reachable from the callback the moment the owner drops its bank entry — clearing a
//! namespace does exactly that while voices are still sounding — so the fixture reproduces it by
//! dropping every owner reference before the voices end.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rbms_audio::{Bus, Command, Mixer, SampleData, rtrb};

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ARMED: AtomicBool = AtomicBool::new(false);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ARMED.load(Ordering::Relaxed) {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ARMED.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const OUT_RATE: u32 = 48_000;
const OUT_CHANNELS: u16 = 2;
const VOICES: usize = 32;
const BUFFER_FRAMES: usize = 512;
const SAMPLE_FRAMES: usize = 12_000;
const RETIRE_SLOTS: usize = 256;
/// Buffers rendered after the last owner reference is dropped, long enough for every voice to reach
/// the end of its sample and hand it back.
const DRAIN_BUFFERS: usize = 64;

fn sample(value: f32) -> Arc<SampleData> {
    Arc::new(SampleData { pcm: vec![value; SAMPLE_FRAMES].into(), channels: 1, rate: OUT_RATE })
}

struct Counts {
    allocations: usize,
    deallocations: usize,
}

/// Run `body` with the counting allocator armed. Nothing else may run on another thread meanwhile,
/// which is why the whole file is one test.
fn measure(body: impl FnOnce()) -> Counts {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    ARMED.store(true, Ordering::Relaxed);
    body();
    ARMED.store(false, Ordering::Relaxed);
    Counts { allocations: ALLOCATIONS.load(Ordering::Relaxed), deallocations: DEALLOCATIONS.load(Ordering::Relaxed) }
}

#[test]
fn the_callback_path_never_reaches_the_allocator() {
    let (retire_tx, mut retire_rx) = rtrb::RingBuffer::<Arc<SampleData>>::new(RETIRE_SLOTS);
    let mut mixer = Mixer::new(OUT_RATE, OUT_CHANNELS, VOICES);
    mixer.set_retire(retire_tx);
    let mut out = vec![0.0f32; BUFFER_FRAMES * OUT_CHANNELS as usize];

    let owned: Vec<Arc<SampleData>> = (0..VOICES).map(|i| sample(0.5 - i as f32 / VOICES as f32)).collect();
    let mut queued: Vec<Command> = owned
        .iter()
        .enumerate()
        .map(|(i, s)| Command::Play { sample: Arc::clone(s), gain: 1.0, pan: 0.0, pitch: 1.0, key: i as u32, at_frame: 0, bus: Bus::ALL[i % Bus::ALL.len()] })
        .collect();

    // Warm up outside the measurement so the fixtures' own allocations are not counted.
    mixer.mix(&mut out);

    let steady = measure(|| {
        for cmd in queued.drain(..) {
            mixer.apply(cmd);
        }
        for _ in 0..8 {
            mixer.mix(&mut out);
        }
        mixer.apply(Command::MasterGain(0.75));
        mixer.apply(Command::BusGain { bus: Bus::Bg, gain: 0.25 });
        mixer.apply(Command::ChartGain(0.9));
        mixer.apply(Command::StopRange { lo_key: 0, hi_key: 4 });
        mixer.apply(Command::StopId { id: 5 });
        mixer.apply(Command::Stop { key: 6 });
        for _ in 0..8 {
            mixer.mix(&mut out);
        }
    });
    assert_eq!(steady.allocations, 0, "the callback path allocated");
    assert_eq!(steady.deallocations, 0, "the callback path freed memory");

    // Now the mixer holds the only references, the way it does after `clear_namespace`.
    drop(owned);
    let retiring = measure(|| {
        for _ in 0..DRAIN_BUFFERS {
            mixer.mix(&mut out);
        }
    });
    assert_eq!(retiring.allocations, 0, "ending a voice allocated");
    assert_eq!(retiring.deallocations, 0, "the callback freed sample PCM instead of handing it back");
    assert_eq!(mixer.retire_overflows(), 0, "the retirement ring overflowed, so the callback had to free samples itself");

    let mut handed_back = 0usize;
    while retire_rx.pop().is_ok() {
        handed_back += 1;
    }
    assert_eq!(handed_back, VOICES, "every sample must come back to the owning thread to be freed");
}
