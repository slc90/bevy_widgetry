#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use bevy_widgetry_waveform::{WaveformMetadata, WaveformSessionKind, WaveformUnit};
use bevy_widgetry_waveform_file::build_recording;
use std::{io, path::PathBuf, time::Instant};

struct Options {
    output: PathBuf,
    seconds: u64,
    rate: u32,
    channels: usize,
    seed: u64,
}

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.to_string())
}

fn options(args: impl Iterator<Item = String>) -> io::Result<Options> {
    let mut result = Options {
        output: "gallery/data/waveform/replay.widx".into(),
        seconds: 60,
        rate: 8000,
        channels: 8,
        seed: 42,
    };
    let mut args = args;
    while let Some(key) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| invalid(format!("missing value for {key}")))?;
        match key.as_str() {
            "--output" => result.output = value.into(),
            "--seconds" => result.seconds = value.parse().map_err(invalid)?,
            "--rate" => result.rate = value.parse().map_err(invalid)?,
            "--channels" => result.channels = value.parse().map_err(invalid)?,
            "--seed" => result.seed = value.parse().map_err(invalid)?,
            _ => return Err(invalid(format!("unknown option {key}"))),
        }
    }
    if result.seconds == 0 || result.rate == 0 || result.channels == 0 {
        return Err(invalid("seconds, rate and channels must be positive"));
    }
    Ok(result)
}

fn signal(channel: usize, frame: u64, rate: u32, seed: u64) -> Option<f32> {
    let rate = u64::from(rate);
    if channel % 4 == 2 && frame % (rate * 7) < (rate / 5).max(1) {
        return None;
    }
    let phase =
        (frame as f64 / rate as f64) * (1.0 + channel as f64 * 0.37 + (seed % 97) as f64 * 0.001);
    if frame % (rate * 5) == (channel as u64 * 113 + seed % rate) % (rate * 5) {
        Some(1.5)
    } else if frame % (rate * 11) == (channel as u64 * 173 + seed % rate + 1) % (rate * 11) {
        Some(-1.5)
    } else {
        Some((phase * std::f64::consts::TAU).sin() as f32 * (0.7 - (channel % 5) as f32 * 0.08))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = options(std::env::args().skip(1))?;
    if options.output.try_exists()? {
        return Err(
            io::Error::new(io::ErrorKind::AlreadyExists, "destination already exists").into(),
        );
    }
    let frames = options
        .seconds
        .checked_mul(u64::from(options.rate))
        .ok_or_else(|| invalid("frame count overflow"))?;
    let raw_bytes = frames
        .checked_mul(options.channels as u64)
        .and_then(|n| n.checked_mul(5))
        .ok_or_else(|| invalid("recording size overflow"))?;
    let metadata = WaveformMetadata {
        channel_count: options.channels,
        channel_labels: (1..=options.channels)
            .map(|i| format!("Disk {i:02}"))
            .collect(),
        sample_rate: options.rate,
        unit: WaveformUnit::Volts,
        kind: WaveformSessionKind::Replay {
            total_frames: frames,
        },
    };
    metadata.validate().map_err(invalid)?;
    if let Some(parent) = options
        .output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    println!(
        "准备生成记录 path={} raw_bytes={} seed={}",
        options.output.display(),
        raw_bytes,
        options.seed
    );
    let start = Instant::now();
    build_recording(&options.output, metadata, |channel, range, out| {
        out.extend(range.map(|frame| signal(channel, frame, options.rate, options.seed)));
        Ok(())
    })?;
    println!(
        "记录及双算法索引生成完成 bytes={} elapsed_s={:.3}",
        std::fs::metadata(&options.output)?.len(),
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn parameters_and_signal_keep_gaps_spikes_and_seed_reproducible() {
        let opts = options(
            ["--seconds", "2", "--channels", "3", "--seed", "8"]
                .into_iter()
                .map(str::to_owned),
        )
        .unwrap();
        assert_eq!((opts.seconds, opts.channels, opts.seed), (2, 3, 8));
        assert!(options(["--rate", "0"].into_iter().map(str::to_owned)).is_err());
        assert!(options(["--unknown", "1"].into_iter().map(str::to_owned)).is_err());
        assert_eq!(signal(0, 42, 8000, 42), Some(1.5));
        assert_eq!(signal(0, 43, 8000, 42), Some(-1.5));
        assert_eq!(signal(2, 0, 8000, 42), None);
        assert!(signal(1, 0, 8000, 42).is_some());
        assert_ne!(signal(1, 2345, 8000, 42), signal(1, 2345, 8000, 43));
    }
}
