use crate::WaveformAmplitude;
use crate::error::failure;
use crate::{WaveformError, WaveformFailure};
use bevy::prelude::*;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct WaveformConfig {
    pub visible_duration_ms: u32,
    pub amplitude_range: WaveformAmplitude,
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct WaveformCursor {
    pub position: Duration,
    pub seek_generation: u64,
}

impl WaveformCursor {
    pub fn seek(&mut self, position: Duration) {
        self.position = position;
        self.seek_generation = self.seek_generation.wrapping_add(1);
    }
}

// Duration has only nanosecond precision.
// Round nanoseconds up at exact frame boundaries because rounding down would lose a frame on round-trip conversion.
pub fn duration_from_frames(frames: u64, sample_rate: u32) -> Result<Duration, WaveformFailure> {
    if !(1..=1_000_000_000).contains(&sample_rate) {
        return Err(failure(
            WaveformError::SampleRate { sample_rate },
            "waveform.time.sample_rate",
        ));
    }
    let nanos = (u128::from(frames) * 1_000_000_000).div_ceil(u128::from(sample_rate));
    Ok(Duration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    ))
}

pub fn sample_boundary(position: Duration, sample_rate: u32) -> Result<u64, WaveformFailure> {
    if !(1..=1_000_000_000).contains(&sample_rate) {
        return Err(failure(
            WaveformError::SampleRate { sample_rate },
            "waveform.time.sample_rate",
        ));
    }
    let frames = position
        .as_nanos()
        .checked_mul(u128::from(sample_rate))
        .map(|value| value / 1_000_000_000);
    frames
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            failure(
                WaveformError::SampleBoundary {
                    position,
                    sample_rate,
                },
                "waveform.time.sample_boundary",
            )
        })
}

impl Default for WaveformConfig {
    fn default() -> Self {
        Self {
            visible_duration_ms: 1000,
            amplitude_range: WaveformAmplitude::default(),
        }
    }
}

impl WaveformConfig {
    pub(crate) fn validate(&self) -> Result<(), WaveformFailure> {
        self.amplitude_range.validate()?;
        if self.visible_duration_ms == 0 {
            return Err(failure(
                WaveformError::Config {
                    visible_duration_ms: self.visible_duration_ms,
                },
                "waveform.config",
            ));
        }
        Ok(())
    }

    pub fn capacity_frames(
        &self,
        sample_rate: u32,
        channels: usize,
    ) -> Result<usize, WaveformFailure> {
        self.validate()?;
        let product = u64::from(sample_rate) * u64::from(self.visible_duration_ms);
        let frames = product / 1000;
        let capacity = usize::try_from(frames).ok();
        let bytes = capacity
            .and_then(|value| value.checked_mul(channels))
            .and_then(|value| value.checked_mul(size_of::<f32>()));
        if !(1..=1_000_000_000).contains(&sample_rate)
            || channels == 0
            || !product.is_multiple_of(1000)
            || frames == 0
            || bytes.is_none_or(|value| value > isize::MAX as usize)
        {
            return Err(failure(
                WaveformError::Capacity {
                    visible_duration_ms: self.visible_duration_ms,
                    sample_rate,
                    channels,
                },
                "waveform.capacity",
            ));
        }
        capacity.ok_or_else(|| {
            failure(
                WaveformError::Capacity {
                    visible_duration_ms: self.visible_duration_ms,
                    sample_rate,
                    channels,
                },
                "waveform.capacity",
            )
        })
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn exact_capacity_and_sample_boundaries() {
        let config = WaveformConfig {
            visible_duration_ms: 10_000,
            ..default()
        };
        assert_eq!(config.capacity_frames(64_000, 64).unwrap(), 640_000);
        for frame in [0, 1, 63, 64, 1067, 640_000, 1_000_001] {
            assert_eq!(
                sample_boundary(duration_from_frames(frame, 64_000).unwrap(), 64_000).unwrap(),
                frame
            );
        }
        assert_eq!(
            sample_boundary(Duration::from_nanos(15_624), 64_000).unwrap(),
            0
        );
    }

    #[test]
    fn rejects_invalid_specifications() {
        for (rate, duration, channels) in [
            (0, 1000, 1),
            (1000, 0, 1),
            (1000, 1000, 0),
            (1, 1, 1),
            (2_000_000_000, 1, 1),
        ] {
            assert!(
                WaveformConfig {
                    visible_duration_ms: duration,
                    amplitude_range: WaveformAmplitude::default()
                }
                .capacity_frames(rate, channels)
                .is_err()
            );
        }
        assert!(sample_boundary(Duration::MAX, u32::MAX).is_err());
        assert!(duration_from_frames(1, 2_000_000_000).is_err());
    }
}
