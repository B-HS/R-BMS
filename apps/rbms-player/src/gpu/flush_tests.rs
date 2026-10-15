//! What a frame that draws nothing does with the writes made for it.
//!
//! The queue holds a write's staging copy until a submission carries it. A window that cannot be
//! shown runs its frames without drawing them, and those frames still upload every movie frame and
//! picture they reach, so each has to hand the queue its writes or they pile up for as long as the
//! window stays hidden. The window-less target draws nothing on [`Gpu::render`] either, which is
//! what these tests run it through.
//!
//! A machine with no adapter cannot run them, and there they pass without checking anything.

use super::Gpu;

/// Size of the window-less target, which nothing is drawn on.
const TARGET_SIZE: u32 = 4;

/// Length of the buffer the write lands in: one copy unit.
const PROBE_BYTES: usize = wgpu::COPY_BUFFER_ALIGNMENT as usize;

/// What the write puts in the buffer, which starts out zeroed.
const WRITTEN_BYTE: u8 = 0xA5;

/// A write made before a frame that draws nothing has reached the GPU once that frame is over.
#[test]
fn a_frame_that_draws_nothing_hands_the_queue_its_writes() {
    let Some(mut gpu) = Gpu::offscreen_if_available(TARGET_SIZE, TARGET_SIZE, (TARGET_SIZE, TARGET_SIZE)) else {
        return;
    };
    let probe = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("flush probe"),
        size: PROBE_BYTES as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let written = [WRITTEN_BYTE; PROBE_BYTES];
    gpu.queue.write_buffer(&probe, 0, &written);

    gpu.render();

    let (sender, mapped) = std::sync::mpsc::channel();
    probe.slice(..).map_async(wgpu::MapMode::Read, move |result| {
        sender.send(result).ok();
    });
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).expect("the device is polled");
    mapped.recv().expect("the mapping answers").expect("the probe maps");
    let held = probe.slice(..).get_mapped_range().to_vec();
    assert_eq!(held, written, "the write was left waiting for a frame that is drawn");
}
