use bevy::prelude::BevyError;
use bevy_widgetry_internal_log::widgetry_error;
use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;

#[derive(Default)]
pub struct PlanarBuffer {
    channels: Vec<Vec<f32>>,
    missing: Vec<Vec<Range<usize>>>,
}

pub trait WaveformSource: Send + Sync + 'static {
    /// Metadata is captured once per session and must describe all reads in that session.
    /// Replacing a recording or channel structure requires establishing a new session.
    fn metadata(&self) -> Result<crate::WaveformMetadata, WaveformReadError>;
    /// Missing ranges are relative to this request and must be sorted, disjoint and in bounds.
    /// Only valid samples must be finite; all channels still supply the full requested length.
    /// Return Pending without blocking, or Ready with the identity of the completed request.
    /// Errors describe the current request; obsolete failures must be discarded by the source.
    /// Samples use the declared source unit; the runtime converts them to volts before commit.
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaveformReadRequest {
    pub id: u64,
    pub reader_id: u64,
    pub session_generation: u64,
    pub target_generation: u64,
    pub range: Range<u64>,
    pub viewport: Range<u64>,
    pub output_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaveformReadStatus {
    Ready(WaveformReadRequest),
    Pending,
}

#[derive(Debug, Clone)]
pub struct WaveformReadError(pub String);

#[derive(Clone, Copy, Debug, Default)]
pub struct ChannelView<'a> {
    pub first: &'a [f32],
    pub second: &'a [f32],
    pub start: u64,
    pub missing_first: &'a [Range<u64>],
    pub missing_second: &'a [Range<u64>],
}

pub(crate) struct PlanarRing {
    channels: Vec<VecDeque<f32>>,
    missing: Vec<VecDeque<Range<u64>>>,
    capacity: usize,
    pub(crate) range: Range<u64>,
}

impl PlanarBuffer {
    pub fn missing_mut(&mut self) -> &mut [Vec<Range<usize>>] {
        &mut self.missing
    }

    pub fn missing(&self) -> &[Vec<Range<usize>>] {
        &self.missing
    }

    pub(crate) fn convert_to_volts(&mut self, unit: crate::WaveformUnit) {
        let factor = unit.volts();
        if factor != 1.0 {
            for channel in &mut self.channels {
                for value in channel {
                    *value *= factor;
                }
            }
        }
    }
    pub fn channels_mut(&mut self) -> &mut [Vec<f32>] {
        &mut self.channels
    }

    pub fn channels(&self) -> &[Vec<f32>] {
        &self.channels
    }

    pub(crate) fn prepare(&mut self, channels: usize, frames: usize) -> Result<(), BevyError> {
        self.channels.resize_with(channels, Vec::new);
        self.missing.resize_with(channels, Vec::new);
        for missing in &mut self.missing {
            missing.clear();
        }
        for channel in &mut self.channels {
            channel.clear();
            channel.try_reserve(frames).map_err(allocation_error)?;
        }
        Ok(())
    }

    pub(crate) fn validate(&self, channels: usize, frames: usize) -> Result<(), WaveformReadError> {
        if self.channels.len() != channels || self.missing.len() != channels {
            return Err(WaveformReadError("source channel count mismatch".into()));
        }
        for (values, missing) in self.channels.iter().zip(&self.missing) {
            if values.len() != frames {
                return Err(WaveformReadError(
                    "source must supply a complete Planar frame range".into(),
                ));
            }
            let mut cursor = 0;
            for gap in missing {
                if gap.start < cursor || gap.start >= gap.end || gap.end > frames {
                    return Err(WaveformReadError(
                        "missing ranges must be ordered, nonoverlapping and within the read".into(),
                    ));
                }
                if values[cursor..gap.start]
                    .iter()
                    .any(|value| !value.is_finite())
                {
                    return Err(WaveformReadError("valid samples must be finite".into()));
                }
                cursor = gap.end;
            }
            if values[cursor..].iter().any(|value| !value.is_finite()) {
                return Err(WaveformReadError("valid samples must be finite".into()));
            }
        }
        Ok(())
    }

    pub(crate) fn allocated_samples(&self) -> usize {
        self.channels.iter().map(Vec::capacity).sum()
    }

    pub(crate) fn missing_bytes(&self) -> usize {
        self.missing
            .iter()
            .map(|gaps| gaps.capacity() * size_of::<Range<usize>>())
            .sum()
    }
}

impl fmt::Display for WaveformReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for WaveformReadError {}

impl<'a> ChannelView<'a> {
    pub fn len(&self) -> usize {
        self.first.len() + self.second.len()
    }

    pub fn is_empty(&self) -> bool {
        self.first.is_empty() && self.second.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<f32> {
        if index >= self.len() || self.valid_ranges(index..index + 1).next().is_none() {
            return None;
        }
        if index < self.first.len() {
            self.first.get(index).copied()
        } else {
            self.second.get(index - self.first.len()).copied()
        }
    }
    pub(crate) fn valid_ranges(
        self,
        range: Range<usize>,
    ) -> impl Iterator<Item = Range<usize>> + 'a {
        let origin = self.start;
        let mut cursor = origin + range.start as u64;
        let end = origin + range.end as u64;
        let first = self.missing_first.partition_point(|gap| gap.end <= cursor);
        let second = self.missing_second.partition_point(|gap| gap.end <= cursor);
        let mut gaps = self.missing_first[first..]
            .iter()
            .chain(&self.missing_second[second..]);
        std::iter::from_fn(move || {
            while cursor < end {
                let left = cursor;
                if let Some(gap) = gaps.next().filter(|gap| gap.start < end) {
                    cursor = cursor.max(gap.end).min(end);
                    if left < gap.start {
                        return Some(
                            (left - origin) as usize..(gap.start.min(end) - origin) as usize,
                        );
                    }
                } else {
                    cursor = end;
                    return Some((left - origin) as usize..(end - origin) as usize);
                }
            }
            None
        })
    }
}

fn allocation_error(error: impl fmt::Display) -> BevyError {
    widgetry_error!(%error, "Waveform buffer allocation 失败");
    BevyError::error(error.to_string())
}

impl PlanarRing {
    pub(crate) fn new(channels: usize, capacity: usize) -> Result<Self, BevyError> {
        let mut data = Vec::with_capacity(channels);
        for _ in 0..channels {
            let mut channel = VecDeque::new();
            channel
                .try_reserve_exact(capacity)
                .map_err(allocation_error)?;
            data.push(channel);
        }
        Ok(Self {
            channels: data,
            missing: (0..channels).map(|_| VecDeque::new()).collect(),
            capacity,
            range: 0..0,
        })
    }

    pub(crate) fn from_staging(
        staging: PlanarBuffer,
        range: Range<u64>,
        capacity: usize,
    ) -> Result<Self, BevyError> {
        let missing = staging
            .missing
            .into_iter()
            .map(|gaps| {
                gaps.into_iter()
                    .map(|gap| range.start + gap.start as u64..range.start + gap.end as u64)
                    .collect()
            })
            .collect();
        let mut channels = Vec::with_capacity(staging.channels.len());
        for samples in staging.channels {
            let mut channel = VecDeque::from(samples);
            channel
                .try_reserve_exact(capacity - channel.len())
                .map_err(allocation_error)?;
            channels.push(channel);
        }
        Ok(Self {
            channels,
            missing,
            capacity,
            range,
        })
    }

    pub(crate) fn append(&mut self, target: Range<u64>, staging: &PlanarBuffer) {
        let remove = (target.start - self.range.start) as usize;
        for (channel, samples) in self.channels.iter_mut().zip(&staging.channels) {
            channel.drain(..remove);
            channel.extend(samples.iter().copied());
        }
        for (missing, incoming) in self.missing.iter_mut().zip(&staging.missing) {
            while missing.front().is_some_and(|gap| gap.end <= target.start) {
                missing.pop_front();
            }
            if let Some(gap) = missing.front_mut() {
                gap.start = gap.start.max(target.start);
            }
            for gap in incoming {
                let start = self.range.end + gap.start as u64;
                let end = self.range.end + gap.end as u64;
                if let Some(last) = missing.back_mut().filter(|last| last.end == start) {
                    last.end = end;
                } else {
                    missing.push_back(start..end);
                }
            }
        }
        self.range = target;
    }

    pub(crate) fn channel(&self, index: usize) -> Option<ChannelView<'_>> {
        self.channels.get(index).map(|channel| {
            let (first, second) = channel.as_slices();
            let (missing_first, missing_second) = self.missing[index].as_slices();
            ChannelView {
                first,
                second,
                start: self.range.start,
                missing_first,
                missing_second,
            }
        })
    }

    pub(crate) fn allocated_samples(&self) -> usize {
        self.channels.iter().map(VecDeque::capacity).sum()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    pub(crate) fn missing_bytes(&self) -> usize {
        self.missing
            .iter()
            .map(|gaps| gaps.capacity() * size_of::<Range<u64>>())
            .sum()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn wrap_preserves_order_and_capacity() {
        let mut ring = PlanarRing::new(2, 8).unwrap();
        let allocation = ring.allocated_samples();
        let mut out = PlanarBuffer::default();
        for end in 1..80u64 {
            out.prepare(2, 1).unwrap();
            for (index, channel) in out.channels_mut().iter_mut().enumerate() {
                channel.push((end - 1) as f32 + index as f32 * 100.0);
            }
            ring.append(end.saturating_sub(8)..end, &out);
            for index in 0..2 {
                let view = ring.channel(index).unwrap();
                let actual: Vec<_> = view.first.iter().chain(view.second).copied().collect();
                let expected: Vec<_> = ring
                    .range
                    .clone()
                    .map(|frame| frame as f32 + index as f32 * 100.0)
                    .collect();
                assert_eq!(actual, expected);
            }
            assert_eq!(ring.allocated_samples(), allocation);
        }
    }
}
