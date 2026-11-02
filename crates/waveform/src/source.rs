use crate::error::failure;
use crate::{WaveformError, WaveformFailure};
use std::collections::VecDeque;
use std::ops::Range;

#[derive(Default)]
pub struct PlanarBuffer {
    channels: Vec<Vec<f32>>,
    missing: Vec<Vec<Range<usize>>>,
}

pub trait WaveformSource: Send + Sync + 'static {
    fn cancel_reads(&self, _reader_id: u64) {}

    fn provides_reduced(&self) -> bool {
        false
    }

    /// Reduced results cover the complete request, use the metadata unit, and retain gaps.
    /// Pending must return promptly, and Ready must carry the exact completed identity.
    fn read_reduced(
        &self,
        _request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        Err(failure(
            WaveformError::UnsupportedRead { reduced: true },
            "waveform.source.read_reduced",
        )
        .into())
    }
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
    pub algorithm: crate::WaveformAlgorithm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaveformReadStatus {
    Ready(WaveformReadRequest),
    Pending,
}

pub enum WaveformReducedReadStatus {
    Ready {
        request: WaveformReadRequest,
        channels: Vec<crate::ReducedChannel>,
    },
    Pending,
}

pub type WaveformReadError = bevy_widgetry_job::JobFailure;

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

    pub(crate) fn prepare(
        &mut self,
        channels: usize,
        frames: usize,
    ) -> Result<(), WaveformFailure> {
        self.channels
            .try_reserve(channels.saturating_sub(self.channels.len()))
            .map_err(|source| {
                allocation_error(source, channels, None, frames, "channel table", channels)
            })?;
        self.missing
            .try_reserve(channels.saturating_sub(self.missing.len()))
            .map_err(|source| {
                allocation_error(source, channels, None, frames, "missing table", channels)
            })?;
        self.channels.resize_with(channels, Vec::new);
        self.missing.resize_with(channels, Vec::new);
        for missing in &mut self.missing {
            missing.clear();
        }
        for (index, channel) in self.channels.iter_mut().enumerate() {
            channel.clear();
            channel.try_reserve(frames).map_err(|source| {
                allocation_error(source, channels, Some(index), frames, "samples", frames)
            })?;
        }
        Ok(())
    }

    pub(crate) fn validate(&self, channels: usize, frames: usize) -> Result<(), WaveformFailure> {
        let invalid = |kind| failure(kind, "waveform.buffer.validate");
        if self.channels.len() != channels || self.missing.len() != channels {
            return Err(invalid(WaveformError::BufferChannels {
                expected: channels,
                values: self.channels.len(),
                missing: self.missing.len(),
            }));
        }
        for (channel, (values, missing)) in self.channels.iter().zip(&self.missing).enumerate() {
            if values.len() != frames {
                return Err(invalid(WaveformError::BufferFrames {
                    channel,
                    expected: frames,
                    actual: values.len(),
                }));
            }
            let mut cursor = 0;
            for (index, gap) in missing.iter().enumerate() {
                if gap.start < cursor || gap.start >= gap.end || gap.end > frames {
                    return Err(invalid(WaveformError::MissingRange {
                        channel,
                        index,
                        range: gap.clone(),
                        previous_end: cursor,
                        frames,
                    }));
                }
                if let Some((offset, value)) = values[cursor..gap.start]
                    .iter()
                    .enumerate()
                    .find(|(_, value)| !value.is_finite())
                {
                    return Err(invalid(WaveformError::NonFinite {
                        channel,
                        frame: cursor + offset,
                        value: *value,
                    }));
                }
                cursor = gap.end;
            }
            if let Some((offset, value)) = values[cursor..]
                .iter()
                .enumerate()
                .find(|(_, value)| !value.is_finite())
            {
                return Err(invalid(WaveformError::NonFinite {
                    channel,
                    frame: cursor + offset,
                    value: *value,
                }));
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

fn allocation_error(
    source: std::collections::TryReserveError,
    channels: usize,
    channel: Option<usize>,
    frames: usize,
    buffer: &'static str,
    elements: usize,
) -> WaveformFailure {
    failure(
        WaveformError::Allocation {
            channels,
            channel,
            frames,
            buffer,
            elements,
            source,
        },
        "waveform.buffer.allocate",
    )
}

impl PlanarRing {
    pub(crate) fn channel_count(&self) -> usize {
        self.channels.len()
    }
    pub(crate) fn new(channels: usize, capacity: usize) -> Result<Self, WaveformFailure> {
        let mut data = Vec::new();
        data.try_reserve_exact(channels).map_err(|source| {
            allocation_error(
                source,
                channels,
                None,
                capacity,
                "ring channel table",
                channels,
            )
        })?;
        let mut missing = Vec::new();
        missing.try_reserve_exact(channels).map_err(|source| {
            allocation_error(
                source,
                channels,
                None,
                capacity,
                "ring missing table",
                channels,
            )
        })?;
        missing.resize_with(channels, VecDeque::new);
        for index in 0..channels {
            let mut channel = VecDeque::new();
            channel.try_reserve_exact(capacity).map_err(|source| {
                allocation_error(
                    source,
                    channels,
                    Some(index),
                    capacity,
                    "ring samples",
                    capacity,
                )
            })?;
            data.push(channel);
        }
        Ok(Self {
            channels: data,
            missing,
            capacity,
            range: 0..0,
        })
    }

    pub(crate) fn from_staging(
        staging: PlanarBuffer,
        range: Range<u64>,
        capacity: usize,
    ) -> Result<Self, WaveformFailure> {
        let count = staging.channels.len();
        let mut missing = Vec::new();
        missing.try_reserve_exact(count).map_err(|source| {
            allocation_error(source, count, None, capacity, "ring missing table", count)
        })?;
        for (index, gaps) in staging.missing.into_iter().enumerate() {
            let mut converted = VecDeque::new();
            converted.try_reserve_exact(gaps.len()).map_err(|source| {
                allocation_error(
                    source,
                    count,
                    Some(index),
                    capacity,
                    "missing ranges",
                    gaps.len(),
                )
            })?;
            converted.extend(
                gaps.into_iter()
                    .map(|gap| range.start + gap.start as u64..range.start + gap.end as u64),
            );
            missing.push(converted);
        }
        let mut channels = Vec::new();
        channels.try_reserve_exact(count).map_err(|source| {
            allocation_error(source, count, None, capacity, "ring channel table", count)
        })?;
        for (index, samples) in staging.channels.into_iter().enumerate() {
            let mut channel = VecDeque::from(samples);
            channel
                .try_reserve_exact(capacity - channel.len())
                .map_err(|source| {
                    allocation_error(
                        source,
                        count,
                        Some(index),
                        capacity,
                        "ring samples",
                        capacity,
                    )
                })?;
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
    fn allocation_failure_keeps_reservation_cause_and_requested_scale() {
        use std::error::Error;
        let mut buffer = PlanarBuffer::default();
        let error = buffer.prepare(1, usize::MAX).unwrap_err();
        assert!(matches!(
            error.kind(),
            WaveformError::Allocation {
                channels: 1,
                channel: Some(0),
                frames: usize::MAX,
                ..
            }
        ));
        assert!(
            error
                .kind()
                .source()
                .unwrap()
                .is::<std::collections::TryReserveError>()
        );
    }

    #[test]
    fn oversized_channel_table_returns_an_allocation_failure() {
        let mut buffer = PlanarBuffer::default();
        let failure = buffer.prepare(usize::MAX, 0).unwrap_err();
        assert!(matches!(
            failure.kind(),
            WaveformError::Allocation {
                channels: usize::MAX,
                frames: 0,
                ..
            }
        ));
    }

    #[test]
    fn buffer_failures_keep_channel_frame_gap_and_sample_values() {
        use std::error::Error;
        let fixtures = [
            PlanarBuffer {
                channels: vec![],
                missing: vec![],
            },
            PlanarBuffer {
                channels: vec![vec![0.0; 3]],
                missing: vec![vec![]],
            },
            PlanarBuffer {
                channels: vec![vec![0.0; 4]],
                missing: vec![vec![1..3, 2..4]],
            },
            PlanarBuffer {
                channels: vec![vec![0.0, f32::NAN, 0.0, 0.0]],
                missing: vec![vec![]],
            },
        ];
        for (index, buffer) in fixtures.into_iter().enumerate() {
            let error = buffer.validate(1, 4).unwrap_err();
            let mut cause: Option<&(dyn Error + 'static)> = Some(&error);
            let mut found = None;
            while let Some(error) = cause {
                if let Some(error) = error.downcast_ref::<crate::WaveformError>() {
                    found = Some(error);
                    break;
                }
                cause = error.source();
            }
            let error = found.expect("structured buffer cause missing");
            match (index, error) {
                (
                    0,
                    crate::WaveformError::BufferChannels {
                        expected: 1,
                        values: 0,
                        missing: 0,
                    },
                ) => {}
                (
                    1,
                    crate::WaveformError::BufferFrames {
                        channel: 0,
                        expected: 4,
                        actual: 3,
                    },
                ) => {}
                (
                    2,
                    crate::WaveformError::MissingRange {
                        channel: 0,
                        index: 1,
                        range,
                        previous_end: 3,
                        frames: 4,
                    },
                ) => assert_eq!(range, &(2..4)),
                (
                    3,
                    crate::WaveformError::NonFinite {
                        channel: 0,
                        frame: 1,
                        value,
                    },
                ) => assert!(value.is_nan()),
                _ => panic!("wrong buffer cause: {error:?}"),
            }
        }
    }

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
