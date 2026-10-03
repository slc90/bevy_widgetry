use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
use std::ops::RangeInclusive;
use std::time::Duration;

#[derive(Clone)]
pub struct WaveformConfig {
    pub sample_rate: u32,
    pub visible_duration_ms: u32,
    pub channel_ranges: Vec<RangeInclusive<f32>>,
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct WaveformCursor {
    pub position: Duration,
}

// Duration 只有 nanosecond 精度。
// 精确 frame boundary 若向下取整，往返转换会少一帧，因此 boundary 的 nanoseconds 向上取整。
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

impl WaveformConfig {
    pub fn capacity_frames(&self) -> Result<usize, BevyError> {
        let product = u64::from(self.sample_rate) * u64::from(self.visible_duration_ms);
        let frames = product / 1000;
        let valid = (1..=1_000_000_000).contains(&self.sample_rate)
            && self.visible_duration_ms > 0
            && !self.channel_ranges.is_empty()
            && product.is_multiple_of(1000)
            && frames > 0
            && self.channel_ranges.iter().all(|range| {
                range.start().is_finite() && range.end().is_finite() && range.start() < range.end()
            });
        let capacity = usize::try_from(frames).ok();
        let bytes = capacity
            .and_then(|value| value.checked_mul(self.channel_ranges.len()))
            .and_then(|value| value.checked_mul(size_of::<f32>()));
        if !valid || bytes.is_none_or(|value| value > isize::MAX as usize) {
            widgetry_error!(
                sample_rate = self.sample_rate,
                duration_ms = self.visible_duration_ms,
                channels = self.channel_ranges.len(),
                "Waveform config 必须具有有效整数容量与有限递增 channel ranges"
            );
            return Err(BevyError::error("Invalid WaveformConfig"));
        }
        capacity.ok_or_else(|| BevyError::error("Waveform capacity overflow"))
    }
}

#[cfg(test)]
// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn exact_capacity_and_sample_boundaries() {
        let config = WaveformConfig {
            sample_rate: 64_000,
            visible_duration_ms: 10_000,
            channel_ranges: vec![-1.0..=1.0; 64],
        };
        assert_eq!(config.capacity_frames().unwrap(), 640_000);
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
        for (rate, duration, ranges) in [
            (0, 1000, vec![-1.0..=1.0]),
            (1000, 0, vec![-1.0..=1.0]),
            (1000, 1000, vec![]),
            (1, 1, vec![-1.0..=1.0]),
            (1000, 1000, vec![1.0..=1.0]),
            (1000, 1000, vec![f32::NAN..=1.0]),
            (1000, 1000, vec![0.0..=f32::INFINITY]),
        ] {
            assert!(
                WaveformConfig {
                    sample_rate: rate,
                    visible_duration_ms: duration,
                    channel_ranges: ranges
                }
                .capacity_frames()
                .is_err()
            );
        }
        assert!(sample_boundary(Duration::MAX, u32::MAX).is_err());
    }

    #[test]
    fn rejects_rates_above_duration_precision() {
        let config = WaveformConfig {
            sample_rate: 2_000_000_000,
            visible_duration_ms: 1,
            channel_ranges: vec![-1.0..=1.0],
        };
        assert!(config.capacity_frames().is_err());
        assert!(duration_from_frames(1, 2_000_000_000).is_err());
        assert!(sample_boundary(Duration::from_nanos(1), 2_000_000_000).is_err());
        assert_eq!(
            sample_boundary(
                duration_from_frames(1, 1_000_000_000).unwrap(),
                1_000_000_000
            )
            .unwrap(),
            1
        );
    }
}
