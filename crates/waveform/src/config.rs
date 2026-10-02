use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
use std::ops::RangeInclusive;
use std::time::Duration;

/// 创建时固定的数据规格；通过 runtime 或 Scene 构造校验后才参与更新。
#[derive(Clone)]
pub struct WaveformConfig {
    /// 每秒 frame 数，范围为 1..=1_000_000_000；上限由 Duration 的 nanosecond 精度决定。
    pub sample_rate: u32,
    /// 必须对应整数 frame 的固定 viewport 时长。
    pub visible_duration_ms: u32,
    /// 按 channel index 排列的有限、严格递增 value range。
    pub channel_ranges: Vec<RangeInclusive<f32>>,
}

/// 外部驱动的时间结束边界；只推进到 source 已确认可读的位置。
/// 这是应用可修改的输入 Component，不是 runtime 已提交 range；推进不保证读取成功。
/// Live / Replay 的推进和暂停由应用 driver 决定，InteractionDisabled 不暂停数据更新。
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct WaveformCursor {
    /// 已确认可读取的半开 frame range 结束时间边界，不是最后一个 sample 的时间戳。
    pub position: Duration,
}

/// 精确 frame boundary 使用向上取整的 nanoseconds，避免往返少一个 frame。
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

/// Duration 转换为向下取整的半开 sample boundary；拒绝 u64 溢出。
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
    /// 校验规格并计算固定容量；失败通过 Result 返回，不进入 runtime。
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
// 局部测试依据项目规则使用断言验证 contract，生产代码仍禁止主动 panic。
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    // 64 kHz 的精确 frame boundary 往返不得少帧，且容量必须与 10 s 一致。
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

    // 非整数容量、空 channel、零速率/时长和非法 range 在构造入口被拒绝。
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

    // Duration 只有 nanosecond 精度，超过 1 GHz 必须拒绝，避免读取未经确认的未来 frame。
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
