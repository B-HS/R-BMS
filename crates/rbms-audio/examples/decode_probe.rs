use std::time::{Duration, Instant};

use rbms_audio::{AudioEngine, decode_bytes};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: decode_probe <audio file> [play]");
    let do_play = args.next().as_deref() == Some("play");

    let bytes = std::fs::read(&path).expect("read");
    let ext = std::path::Path::new(&path).extension().and_then(|e| e.to_str());
    let n = bytes.len();

    let dec = decode_bytes(bytes.clone(), ext).expect("decode");
    let frames = dec.samples.len() / dec.channels.max(1) as usize;
    let dur = frames as f64 / dec.rate.max(1) as f64;
    let peak = dec.samples.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
    println!("file     : {path} ({n} bytes, ext={ext:?})");
    println!("channels : {}", dec.channels);
    println!("rate     : {} Hz", dec.rate);
    println!("frames   : {frames}  ({dur:.3}s)");
    println!("peak amp : {peak:.4}");

    if !do_play {
        return;
    }

    match AudioEngine::new() {
        Ok(mut engine) => {
            println!("\nengine   : opened, device rate {} Hz", engine.out_rate());
            engine.load(1, bytes, ext).expect("load");
            engine.play(1, 1.0, 0.0, 1.0, 0);
            println!("playing... watching sample clock for 400ms");
            let start = Instant::now();
            let mut last = -1i64;
            while start.elapsed() < Duration::from_millis(400) {
                let us = engine.clock_us();
                if us / 50_000 != last {
                    last = us / 50_000;
                    println!("  clock = {:.3}s (frames {})", us as f64 / 1e6, engine.clock_frames());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let final_us = engine.clock_us();
            println!("final clock = {:.3}s", final_us as f64 / 1e6);
            if final_us > 100_000 {
                println!("RESULT   : OK — sample clock advanced in real time (stream running)");
            } else {
                println!("RESULT   : clock did not advance (stream may not be running)");
            }
        }
        Err(e) => println!("\nengine   : could not open ({e}) — decode path still verified"),
    }
}
