#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");
const GAP_CHANNEL_CYCLE: usize = 4;
const GAP_CHANNEL_INDEX: usize = 2;
const GAP_PERIOD_SECONDS: u64 = 7;
const GAP_DURATION_DIVISOR: u64 = 5;
const CHANNEL_FREQUENCY_STEP: f64 = 0.37;
const SEED_FREQUENCY_CYCLE: u64 = 97;
const SEED_FREQUENCY_STEP: f64 = 0.001;
const POSITIVE_SPIKE_PERIOD_SECONDS: u64 = 5;
const POSITIVE_SPIKE_CHANNEL_OFFSET: u64 = 113;
const NEGATIVE_SPIKE_PERIOD_SECONDS: u64 = 11;
const NEGATIVE_SPIKE_CHANNEL_OFFSET: u64 = 173;
const SPIKE_AMPLITUDE: f32 = 1.5;
const BASE_AMPLITUDE: f32 = 0.7;
const AMPLITUDE_CHANNEL_CYCLE: usize = 5;
const AMPLITUDE_CHANNEL_STEP: f32 = 0.08;
const RAW_RECORD_BYTES: u64 = 5;

use bevy_widgetry::waveform::{WaveformMetadata, WaveformSessionKind, WaveformUnit};
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
    if channel % GAP_CHANNEL_CYCLE == GAP_CHANNEL_INDEX
        && frame % (rate * GAP_PERIOD_SECONDS) < (rate / GAP_DURATION_DIVISOR).max(1)
    {
        return None;
    }
    let phase = (frame as f64 / rate as f64)
        * (1.0
            + channel as f64 * CHANNEL_FREQUENCY_STEP
            + (seed % SEED_FREQUENCY_CYCLE) as f64 * SEED_FREQUENCY_STEP);
    if frame % (rate * POSITIVE_SPIKE_PERIOD_SECONDS)
        == (channel as u64 * POSITIVE_SPIKE_CHANNEL_OFFSET + seed % rate)
            % (rate * POSITIVE_SPIKE_PERIOD_SECONDS)
    {
        Some(SPIKE_AMPLITUDE)
    } else if frame % (rate * NEGATIVE_SPIKE_PERIOD_SECONDS)
        == (channel as u64 * NEGATIVE_SPIKE_CHANNEL_OFFSET + seed % rate + 1)
            % (rate * NEGATIVE_SPIKE_PERIOD_SECONDS)
    {
        Some(-SPIKE_AMPLITUDE)
    } else {
        Some(
            (phase * std::f64::consts::TAU).sin() as f32
                * (BASE_AMPLITUDE
                    - (channel % AMPLITUDE_CHANNEL_CYCLE) as f32 * AMPLITUDE_CHANNEL_STEP),
        )
    }
}

fn main() -> std::process::ExitCode {
    bevy_widgetry_test_utils::benchmark::artifact::finish(run())
}

fn run() -> bevy::prelude::Result {
    use bevy_widgetry_test_utils::benchmark::artifact::error;
    let options = options(std::env::args().skip(1)).map_err(error)?;
    if options.output.try_exists().map_err(error)? {
        return Err(error(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination already exists",
        )));
    }
    let frames = options
        .seconds
        .checked_mul(u64::from(options.rate))
        .ok_or_else(|| error(invalid("frame count overflow")))?;
    let raw_bytes = frames
        .checked_mul(options.channels as u64)
        .and_then(|n| n.checked_mul(RAW_RECORD_BYTES))
        .ok_or_else(|| error(invalid("recording size overflow")))?;
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
    metadata
        .validate()
        .map_err(|failure| failure.into_bevy_error())?;
    if let Some(parent) = options
        .output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(error)?;
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
    })
    .map_err(|failure| failure.into_bevy_error())?;
    println!(
        "记录及双算法索引生成完成 bytes={} elapsed_s={:.3}",
        std::fs::metadata(&options.output).map_err(error)?.len(),
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
        const EXPECTED_SPIKE_AMPLITUDE: f32 = 1.5;
        let opts = options(
            ["--seconds", "2", "--channels", "3", "--seed", "8"]
                .into_iter()
                .map(str::to_owned),
        )
        .unwrap();
        assert_eq!((opts.seconds, opts.channels, opts.seed), (2, 3, 8));
        assert!(options(["--rate", "0"].into_iter().map(str::to_owned)).is_err());
        assert!(options(["--unknown", "1"].into_iter().map(str::to_owned)).is_err());
        assert_eq!(signal(0, 42, 8000, 42), Some(EXPECTED_SPIKE_AMPLITUDE));
        assert_eq!(signal(0, 43, 8000, 42), Some(-EXPECTED_SPIKE_AMPLITUDE));
        assert_eq!(signal(2, 0, 8000, 42), None);
        assert!(signal(1, 0, 8000, 42).is_some());
        assert_ne!(signal(1, 2345, 8000, 42), signal(1, 2345, 8000, 43));
    }
}
