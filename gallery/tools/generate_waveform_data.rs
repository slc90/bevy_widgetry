//! 手动运行：rustc gallery/tools/generate_waveform_data.rs -o target/generate_waveform_data.exe
//! 然后 target/generate_waveform_data.exe；默认输出固定 Gallery replay asset。

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
    out.write_all(b"WFRM")?;
    for value in [1_u32, 8000, 4] {
        out.write_all(&value.to_le_bytes())?;
    }
    out.write_all(&240000_u64.to_le_bytes())?;
    for channel in 0..4 {
        for frame in 0..240000 {
            let phase = (frame % (8000 * (channel + 1))) as f32 / (8000 * (channel + 1)) as f32;
            let value = match channel {
                0 => (phase * std::f32::consts::TAU).sin(),
                1 => {
                    if phase < 0.5 {
                        0.8
                    } else {
                        -0.8
                    }
                }
                2 => phase * 2.0 - 1.0,
                _ => (1.0 - (phase * 2.0 - 1.0).abs() * 2.0) * 0.6,
            };
            out.write_all(&value.to_le_bytes())?;
        }
    }
    out.flush()
}
