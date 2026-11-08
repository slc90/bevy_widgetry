const MAGIC: &[u8; 4] = b"WFRM";
const FORMAT_VERSION: u32 = 1;
const SAMPLE_RATE: u32 = 8000;
const CHANNEL_COUNT: u32 = 4;
const TOTAL_FRAMES: u64 = 240000;
const SQUARE_DUTY_CYCLE: f32 = 0.5;
const SQUARE_AMPLITUDE: f32 = 0.8;
const TRIANGLE_AMPLITUDE: f32 = 0.6;

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("gallery/src/assets/waveform/basic_replay.wfrm"));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = BufWriter::new(File::create(path)?);
    out.write_all(MAGIC)?;
    for value in [FORMAT_VERSION, SAMPLE_RATE, CHANNEL_COUNT] {
        out.write_all(&value.to_le_bytes())?;
    }
    out.write_all(&TOTAL_FRAMES.to_le_bytes())?;
    for channel in 0..CHANNEL_COUNT as i32 {
        for frame in 0..TOTAL_FRAMES as i32 {
            let phase = (frame % (SAMPLE_RATE as i32 * (channel + 1))) as f32 / (SAMPLE_RATE as i32 * (channel + 1)) as f32;
            let value = match channel {
                0 => (phase * std::f32::consts::TAU).sin(),
                1 => {
                    if phase < SQUARE_DUTY_CYCLE {
                        SQUARE_AMPLITUDE
                    } else {
                        -SQUARE_AMPLITUDE
                    }
                }
                2 => phase * 2.0 - 1.0,
                _ => (1.0 - (phase * 2.0 - 1.0).abs() * 2.0) * TRIANGLE_AMPLITUDE,
            };
            out.write_all(&value.to_le_bytes())?;
        }
    }
    out.flush()
}
