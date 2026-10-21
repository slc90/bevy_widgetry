use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_error;
use std::ops::RangeInclusive;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub struct WaveformConfig {
    pub visible_duration_ms: u32,
    pub amplitude_range: RangeInclusive<f32>,
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct WaveformCursor {
    pub position: Duration,
}

// Duration has only nanosecond precision.
// Round nanoseconds up at exact frame boundaries because rounding down would lose a frame on round-trip conversion.
pub fn duration_from_frames(frames: u64, sample_rate: u32) -> Result<Duration, BevyError> {
    if !(1..=1_000_000_000).contains(&sample_rate) {
        widgetry_error!(sample_rate, "Waveform sample_rate 必须适配 Duration 精度");
        return Err(BevyError::error(
            "sample_rate must be within 1..=1_000_000_000",
        ));
    }
    let nanos = (u128::from(frames) * 1_000_000_000).div_ceil(u128::from(sample_rate));
    Ok(Duration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    ))
}

pub fn sample_boundary(position: Duration, sample_rate: u32) -> Result<u64, BevyError> {
    if !(1..=1_000_000_000).contains(&sample_rate) {
        widgetry_error!(sample_rate, "Waveform sample_rate 必须适配 Duration 精度");
        return Err(BevyError::error(
            "sample_rate must be within 1..=1_000_000_000",
        ));
    }
    let frames = position
        .as_nanos()
        .checked_mul(u128::from(sample_rate))
        .map(|value| value / 1_000_000_000);
    frames
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(|| {
            widgetry_error!(?position, sample_rate, "Waveform sample boundary 溢出");
            BevyError::error("Waveform sample boundary overflow")
        })
}

impl Default for WaveformConfig {
    fn default() -> Self {
        Self {
            visible_duration_ms: 1000,
            amplitude_range: -1.0..=1.0,
        }
    }
}

impl WaveformConfig {
    pub(crate) fn validate(&self) -> Result<(), BevyError> {
        if self.visible_duration_ms == 0
            || !self.amplitude_range.start().is_finite()
            || !self.amplitude_range.end().is_finite()
            || self.amplitude_range.start() >= self.amplitude_range.end()
        {
            widgetry_error!(?self, "Waveform 显示配置无效");
            return Err(BevyError::error("Invalid WaveformConfig"));
        }
        Ok(())
    }

    pub fn capacity_frames(&self, sample_rate: u32, channels: usize) -> Result<usize, BevyError> {
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
            widgetry_error!(sample_rate, channels, "Waveform 容量无效");
            return Err(BevyError::error("Invalid Waveform capacity"));
        }
        capacity.ok_or_else(|| BevyError::error("Waveform capacity overflow"))
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
        for (rate, duration, channels, range) in [
            (0, 1000, 1, -1.0..=1.0),
            (1000, 0, 1, -1.0..=1.0),
            (1000, 1000, 0, -1.0..=1.0),
            (1, 1, 1, -1.0..=1.0),
            (1000, 1000, 1, 1.0..=1.0),
            (1000, 1000, 1, f32::NAN..=1.0),
            (1000, 1000, 1, 0.0..=f32::INFINITY),
            (2_000_000_000, 1, 1, -1.0..=1.0),
        ] {
            assert!(
                WaveformConfig {
                    visible_duration_ms: duration,
                    amplitude_range: range
                }
                .capacity_frames(rate, channels)
                .is_err()
            );
        }
        assert!(sample_boundary(Duration::MAX, u32::MAX).is_err());
        assert!(duration_from_frames(1, 2_000_000_000).is_err());
    }
}
