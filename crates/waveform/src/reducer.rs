use crate::ChannelView;
use std::collections::VecDeque;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformPoint {
    pub sample: u64,
    pub value: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaveformSpan {
    pub samples: RangeBoundary,
    pub min: f32,
    pub max: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeBoundary {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReducedChannel {
    Polyline(Vec<WaveformPoint>),
    Envelope(Vec<WaveformSpan>),
}

pub struct ReductionInput<'a> {
    pub channels: &'a [ChannelView<'a>],
    pub buffered: Range<u64>,
    pub viewport: Range<u64>,
    pub appended: Range<u64>,
    pub output_len: usize,
    pub rebuild: bool,
}

pub trait WaveformReducer: Send + Sync + 'static {
    fn reduce(
        &mut self,
        input: ReductionInput<'_>,
        output: &mut Vec<ReducedChannel>,
    ) -> ReductionStats;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReductionStats {
    pub visited_samples: usize,
    pub cached_buckets: usize,
    pub allocated_buckets: usize,
}

#[derive(Default)]
pub struct MinMaxReducer {
    bucket_size: u64,
    buckets: Vec<VecDeque<Bucket>>,
}

struct Bucket {
    index: u64,
    min: f32,
    max: f32,
}

fn extrema(view: ChannelView<'_>, start: usize, end: usize) -> (f32, f32) {
    let split = view.first.len();
    let first = &view.first[start.min(split)..end.min(split)];
    let second = &view.second[start.saturating_sub(split)..end.saturating_sub(split)];
    first
        .iter()
        .chain(second)
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), &value| {
            (min.min(value), max.max(value))
        })
}

impl WaveformReducer for MinMaxReducer {
    fn reduce(
        &mut self,
        input: ReductionInput<'_>,
        output: &mut Vec<ReducedChannel>,
    ) -> ReductionStats {
        let mut stats = ReductionStats::default();
        let capacity = input.viewport.end - input.viewport.start;
        if input.output_len == 0 {
            return stats;
        }
        let polyline = capacity <= input.output_len as u64;
        let size = capacity.div_ceil(input.output_len as u64).max(1);
        let rebuild =
            input.rebuild || self.bucket_size != size || self.buckets.len() != input.channels.len();
        self.bucket_size = size;
        self.buckets
            .resize_with(input.channels.len(), VecDeque::new);
        output.resize_with(
            input.channels.len(),
            || ReducedChannel::Envelope(Vec::new()),
        );
        for (channel_index, &view) in input.channels.iter().enumerate() {
            let cached = &mut self.buckets[channel_index];
            if rebuild {
                cached.clear();
            }
            if polyline {
                if !matches!(output[channel_index], ReducedChannel::Polyline(_)) {
                    output[channel_index] = ReducedChannel::Polyline(Vec::new());
                }
                if let ReducedChannel::Polyline(points) = &mut output[channel_index] {
                    points.clear();
                    points.extend(view.first.iter().chain(view.second).enumerate().map(
                        |(index, &value)| WaveformPoint {
                            sample: input.buffered.start + index as u64,
                            value,
                        },
                    ));
                    stats.visited_samples += view.len();
                }
                continue;
            }
            let first_bucket = input.buffered.start / size;
            while cached
                .front()
                .is_some_and(|bucket| bucket.index < first_bucket)
            {
                cached.pop_front();
            }
            let appended = if rebuild {
                input.buffered.clone()
            } else {
                input.appended.clone()
            };
            let mut start = appended.start;
            while start < appended.end {
                let index = start / size;
                let end = appended.end.min(start.saturating_add(size - start % size));
                let (min, max) = extrema(
                    view,
                    (start - input.buffered.start) as usize,
                    (end - input.buffered.start) as usize,
                );
                stats.visited_samples += (end - start) as usize;
                if let Some(bucket) = cached.back_mut().filter(|bucket| bucket.index == index) {
                    bucket.min = bucket.min.min(min);
                    bucket.max = bucket.max.max(max);
                } else {
                    cached.push_back(Bucket { index, min, max });
                }
                start = end;
            }
            if !matches!(output[channel_index], ReducedChannel::Envelope(_)) {
                output[channel_index] = ReducedChannel::Envelope(Vec::new());
            }
            if let ReducedChannel::Envelope(spans) = &mut output[channel_index] {
                spans.clear();
                for bucket in cached.iter() {
                    let bucket_start = bucket.index * size;
                    let start = bucket_start.max(input.buffered.start);
                    let end = bucket_start.saturating_add(size).min(input.buffered.end);
                    let (min, max) = if bucket_start < input.buffered.start {
                        stats.visited_samples += (end - start) as usize;
                        extrema(view, 0, (end - input.buffered.start) as usize)
                    } else {
                        (bucket.min, bucket.max)
                    };
                    spans.push(WaveformSpan {
                        samples: RangeBoundary { start, end },
                        min,
                        max,
                    });
                }
            }
            stats.cached_buckets += cached.len();
            stats.allocated_buckets += cached.capacity();
        }
        stats
    }
}

#[cfg(test)]
// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn run(
        reducer: &mut MinMaxReducer,
        samples: &[f32],
        range: Range<u64>,
        append: Range<u64>,
        width: usize,
        rebuild: bool,
        output: &mut Vec<ReducedChannel>,
    ) -> ReductionStats {
        let split = samples.len() / 2;
        let views = [ChannelView {
            first: &samples[..split],
            second: &samples[split..],
        }];
        reducer.reduce(
            ReductionInput {
                channels: &views,
                buffered: range.clone(),
                viewport: range.end.saturating_sub(16)..range.end.max(16),
                appended: append,
                output_len: width,
                rebuild,
            },
            output,
        )
    }

    #[test]
    fn initial_density_uses_viewport_not_arrived_samples() {
        let mut reducer = MinMaxReducer::default();
        let mut output = Vec::new();
        run(&mut reducer, &[0.2, -0.8], 0..2, 0..2, 4, true, &mut output);
        assert_eq!(
            output,
            vec![ReducedChannel::Envelope(vec![WaveformSpan {
                samples: RangeBoundary { start: 0, end: 2 },
                min: -0.8,
                max: 0.2
            }])]
        );
    }

    #[test]
    fn incremental_matches_reference_with_partial_edges() {
        let mut reducer = MinMaxReducer::default();
        let mut output = Vec::new();
        for end in 1..120u64 {
            let range = end.saturating_sub(16)..end;
            let samples: Vec<_> = range
                .clone()
                .map(|i| if i % 7 == 0 { 4.0 } else { -((i % 5) as f32) })
                .collect();
            let stats = run(
                &mut reducer,
                &samples,
                range.clone(),
                end - 1..end,
                4,
                end == 1,
                &mut output,
            );
            let mut reference = MinMaxReducer::default();
            let mut expected = Vec::new();
            run(
                &mut reference,
                &samples,
                range.clone(),
                range,
                4,
                true,
                &mut expected,
            );
            assert_eq!(output, expected, "end={end}");
            assert!(stats.visited_samples <= 5);
            assert!(stats.cached_buckets <= 5);
        }
        let samples: Vec<_> = (0..16).map(|i| i as f32).collect();
        run(&mut reducer, &samples, 0..16, 0..16, 32, true, &mut output);
        assert!(matches!(&output[0], ReducedChannel::Polyline(points) if points.len() == 16));
    }

    #[test]
    fn globally_aligned_buckets_cover_every_sample_once() {
        let mut reducer = MinMaxReducer::default();
        let mut output = Vec::new();
        let samples = [
            9.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, -7.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        ];
        run(&mut reducer, &samples, 0..16, 0..16, 3, true, &mut output);
        let ReducedChannel::Envelope(spans) = &output[0] else {
            panic!("expected envelope");
        };
        assert_eq!(
            spans
                .iter()
                .map(|span| (span.samples, span.min, span.max))
                .collect::<Vec<_>>(),
            [
                (RangeBoundary { start: 0, end: 6 }, 1.0, 9.0),
                (RangeBoundary { start: 6, end: 12 }, -7.0, 11.0),
                (RangeBoundary { start: 12, end: 16 }, 12.0, 15.0)
            ]
        );
        let next: Vec<_> = samples[1..].iter().copied().chain([16.0]).collect();
        run(&mut reducer, &next, 1..17, 16..17, 3, false, &mut output);
        let ReducedChannel::Envelope(spans) = &output[0] else {
            panic!("expected envelope");
        };
        assert_eq!(spans[0].samples, RangeBoundary { start: 1, end: 6 });
        assert_eq!((spans[0].min, spans[0].max), (1.0, 5.0));
        assert_eq!(
            spans
                .iter()
                .map(|span| span.samples.end - span.samples.start)
                .sum::<u64>(),
            16
        );
    }
}
