use crate::{
    ChannelView, ReducedChannel, ReductionInput, ReductionStats, WaveformPoint, WaveformReducer,
};
use std::collections::VecDeque;

#[derive(Default)]
pub struct LttbReducer {
    size: u64,
    channels: Vec<VecDeque<Bucket>>,
}

struct Bucket {
    index: u64,
    segments: Vec<Segment>,
}

struct Segment {
    first: WaveformPoint,
    last: WaveformPoint,
    sum: f64,
    count: u64,
    lower: Vec<WaveformPoint>,
    upper: Vec<WaveformPoint>,
}

fn cross(a: WaveformPoint, b: WaveformPoint, c: WaveformPoint) -> f64 {
    (b.sample - a.sample) as f64 * (f64::from(c.value) - f64::from(a.value))
        - (c.sample - a.sample) as f64 * (f64::from(b.value) - f64::from(a.value))
}

impl Segment {
    fn new(point: WaveformPoint) -> Self {
        Self {
            first: point,
            last: point,
            sum: 0.0,
            count: 0,
            lower: Vec::new(),
            upper: Vec::new(),
        }
    }

    fn push(&mut self, point: WaveformPoint) {
        for (hull, sign) in [(&mut self.lower, 1.0), (&mut self.upper, -1.0)] {
            while hull.len() >= 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) * sign <= 0.0
            {
                hull.pop();
            }
            hull.push(point);
        }
        self.last = point;
        self.sum += f64::from(point.value);
        self.count += 1;
    }

    fn average(&self, origin: u64) -> (f64, f64) {
        (
            ((self.first.sample - origin) as f64 + (self.last.sample - origin) as f64) * 0.5,
            self.sum / self.count as f64,
        )
    }

    fn representative(&self, previous: WaveformPoint, next: (f64, f64)) -> WaveformPoint {
        let score = |point: WaveformPoint| {
            (point.sample - previous.sample) as f64 * (next.1 - f64::from(previous.value))
                - next.0 * (f64::from(point.value) - f64::from(previous.value))
        };
        let mut best = self.first;
        let mut area = score(best).abs();
        // Each signed linear functional is concave on one hull chain.
        // Binary search avoids rescanning convex signals whose hull contains every raw point.
        for (hull, sign) in [(&self.lower, 1.0), (&self.upper, -1.0)] {
            let sign = if next.0 >= 0.0 { sign } else { -sign };
            let mut left = 0;
            let mut right = hull.len().saturating_sub(1);
            while left < right {
                let middle = (left + right) / 2;
                if score(hull[middle]) * sign < score(hull[middle + 1]) * sign {
                    left = middle + 1;
                } else {
                    right = middle;
                }
            }
            if let Some(&candidate) = hull.get(left) {
                let candidate_area = score(candidate).abs();
                if candidate_area > area || candidate_area == area && candidate.sample < best.sample
                {
                    best = candidate;
                    area = candidate_area;
                }
            }
        }
        best
    }
}

fn append(bucket: &mut Bucket, view: ChannelView<'_>, start: usize, end: usize) {
    for valid in view.valid_ranges(start..end) {
        if bucket
            .segments
            .last()
            .is_none_or(|segment| segment.last.sample + 1 != view.start + valid.start as u64)
        {
            let index = valid.start;
            let point = WaveformPoint {
                sample: view.start + index as u64,
                value: value(view, index),
                break_before: false,
            };
            bucket.segments.push(Segment::new(point));
        }
        if let Some(segment) = bucket.segments.last_mut() {
            for index in valid {
                segment.push(WaveformPoint {
                    sample: view.start + index as u64,
                    value: value(view, index),
                    break_before: false,
                });
            }
        }
    }
}

fn value(view: ChannelView<'_>, index: usize) -> f32 {
    if index < view.first.len() {
        view.first[index]
    } else {
        view.second[index - view.first.len()]
    }
}

impl WaveformReducer for LttbReducer {
    fn reduce(
        &mut self,
        input: ReductionInput<'_>,
        output: &mut Vec<ReducedChannel>,
    ) -> ReductionStats {
        let mut stats = ReductionStats::default();
        if input.output_len == 0 {
            return stats;
        }
        let size = (input.viewport.end - input.viewport.start)
            .div_ceil(input.output_len as u64)
            .max(1);
        let rebuild =
            input.rebuild || self.size != size || self.channels.len() != input.channels.len();
        self.size = size;
        self.channels
            .resize_with(input.channels.len(), VecDeque::new);
        output.resize_with(
            input.channels.len(),
            || ReducedChannel::Polyline(Vec::new()),
        );
        if input.channels.len() < 8 {
            for ((cached, &view), channel) in self
                .channels
                .iter_mut()
                .zip(input.channels)
                .zip(output.iter_mut())
            {
                let result = reduce_channel(&input, size, rebuild, cached, view, channel);
                stats.visited_samples += result.visited_samples;
                stats.cached_buckets += result.cached_buckets;
                stats.allocated_buckets += result.allocated_buckets;
            }
        } else {
            let results = bevy::tasks::ComputeTaskPool::get_or_init(bevy::tasks::TaskPool::new)
                .scope(|scope| {
                    for ((cached, views), channels) in self
                        .channels
                        .chunks_mut(4)
                        .zip(input.channels.chunks(4))
                        .zip(output.chunks_mut(4))
                    {
                        let input = &input;
                        scope.spawn(async move {
                            let mut stats = ReductionStats::default();
                            for ((cache, &view), channel) in
                                cached.iter_mut().zip(views).zip(channels)
                            {
                                let result =
                                    reduce_channel(input, size, rebuild, cache, view, channel);
                                stats.visited_samples += result.visited_samples;
                                stats.cached_buckets += result.cached_buckets;
                                stats.allocated_buckets += result.allocated_buckets;
                            }
                            stats
                        });
                    }
                });
            for result in results {
                stats.visited_samples += result.visited_samples;
                stats.cached_buckets += result.cached_buckets;
                stats.allocated_buckets += result.allocated_buckets;
            }
        }
        stats
    }
}

fn reduce_channel(
    input: &ReductionInput<'_>,
    size: u64,
    rebuild: bool,
    cached: &mut VecDeque<Bucket>,
    view: ChannelView<'_>,
    channel: &mut ReducedChannel,
) -> ReductionStats {
    let mut stats = ReductionStats::default();
    if rebuild {
        cached.clear();
    }
    while cached
        .front()
        .is_some_and(|bucket| bucket.index < input.buffered.start / size)
    {
        cached.pop_front();
    }
    let mut start = if rebuild {
        input.buffered.start
    } else {
        input.appended.start
    };
    // The leftmost retained bucket may contain expired points and must be clipped exactly.
    if let Some(front) = cached
        .front_mut()
        .filter(|bucket| bucket.index * size < input.buffered.start)
    {
        let end = front
            .index
            .saturating_add(1)
            .saturating_mul(size)
            .min(input.buffered.end)
            .min(start);
        front.segments.clear();
        append(front, view, 0, (end - input.buffered.start) as usize);
        stats.visited_samples += (end - input.buffered.start) as usize;
    }
    while start < input.appended.end {
        let index = start / size;
        let end = input
            .appended
            .end
            .min(start.saturating_add(size - start % size));
        if cached.back().is_none_or(|bucket| bucket.index != index) {
            cached.push_back(Bucket {
                index,
                segments: Vec::new(),
            });
        }
        if let Some(bucket) = cached.back_mut() {
            append(
                bucket,
                view,
                (start - input.buffered.start) as usize,
                (end - input.buffered.start) as usize,
            );
        }
        stats.visited_samples += (end - start) as usize;
        start = end;
    }
    if !matches!(channel, ReducedChannel::Polyline(_)) {
        *channel = ReducedChannel::Polyline(Vec::new());
    }
    if let ReducedChannel::Polyline(points) = channel {
        points.clear();
        let segments: Vec<_> = cached.iter().flat_map(|bucket| &bucket.segments).collect();
        let mut begin = 0;
        while begin < segments.len() {
            let mut end = begin + 1;
            while end < segments.len()
                && segments[end - 1].last.sample + 1 == segments[end].first.sample
            {
                end += 1;
            }
            let first = segments[begin].first;
            points.push(WaveformPoint {
                break_before: true,
                ..first
            });
            let mut previous = first;
            for index in begin..end {
                let last = segments[end - 1].last;
                let next = if index + 1 < end {
                    segments[index + 1].average(previous.sample)
                } else {
                    (
                        (last.sample - previous.sample) as f64,
                        f64::from(last.value),
                    )
                };
                let point = segments[index].representative(previous, next);
                if point.sample > previous.sample {
                    points.push(point);
                    previous = point;
                }
            }
            let last = segments[end - 1].last;
            if last.sample > previous.sample {
                points.push(last);
            }
            begin = end;
        }
    }
    stats.cached_buckets += cached.len();
    stats.allocated_buckets += cached.capacity();
    stats
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn final_time_bucket_clips_without_overflowing_the_sample_domain() {
        let mut incremental = LttbReducer::default();
        for end in u64::MAX - 10..=u64::MAX {
            let start = end - 100;
            let values: Vec<_> = (start..end).map(|i| (i % 5) as f32).collect();
            let views = [ChannelView {
                first: &values,
                start,
                ..Default::default()
            }];
            let mut actual = Vec::new();
            let mut expected = Vec::new();
            for (reducer, rebuild, output) in [
                (&mut incremental, end == u64::MAX - 10, &mut actual),
                (&mut LttbReducer::default(), true, &mut expected),
            ] {
                reducer.reduce(
                    ReductionInput {
                        channels: &views,
                        buffered: start..end,
                        viewport: start..end,
                        appended: if rebuild { start..end } else { end - 1..end },
                        output_len: 3,
                        rebuild,
                    },
                    output,
                );
            }
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn parallel_channels_keep_real_points_and_lttb_can_omit_an_extremum() {
        let samples = [0.0, 100.0, -101.0, 0.0];
        let views = [ChannelView {
            first: &samples,
            ..Default::default()
        }; 16];
        let mut output = Vec::new();
        let stats = LttbReducer::default().reduce(
            ReductionInput {
                channels: &views,
                buffered: 0..4,
                viewport: 0..4,
                appended: 0..4,
                output_len: 1,
                rebuild: true,
            },
            &mut output,
        );
        assert_eq!(stats.visited_samples, 64);
        assert_eq!(output.len(), 16);
        for channel in output {
            let ReducedChannel::Polyline(points) = channel else {
                panic!("polyline");
            };
            assert_eq!(
                points
                    .iter()
                    .map(|p| (p.sample, p.value))
                    .collect::<Vec<_>>(),
                [(0, 0.0), (2, -101.0), (3, 0.0)]
            );
            assert!(points[0].break_before);
            assert!(!points[1].break_before);
        }
    }

    #[test]
    fn hull_selection_matches_all_raw_triangle_candidates_with_large_sample_origins() {
        for seed in 0..40u64 {
            let origin = (1u64 << 58) + seed;
            let previous = WaveformPoint {
                sample: origin,
                value: -0.7,
                break_before: false,
            };
            let next = (103.5, 0.4);
            let mut random = seed + 1;
            let values: Vec<_> = (1..100)
                .map(|i| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    WaveformPoint {
                        sample: origin + i,
                        value: if seed == 0 {
                            (i * i) as f32
                        } else {
                            (random >> 40) as f32 / 1000.0
                        },
                        break_before: false,
                    }
                })
                .collect();
            let mut segment = Segment::new(values[0]);
            for &point in &values {
                segment.push(point);
            }
            let area = |p: WaveformPoint| {
                ((p.sample - origin) as f64 * (next.1 - f64::from(previous.value))
                    - next.0 * (f64::from(p.value) - f64::from(previous.value)))
                .abs()
            };
            let expected = values
                .iter()
                .copied()
                .reduce(|a, b| if area(b) > area(a) { b } else { a })
                .unwrap();
            assert_eq!(segment.representative(previous, next), expected);
        }
    }

    fn run(
        reducer: &mut LttbReducer,
        start: u64,
        end: u64,
        previous: u64,
        width: usize,
        rebuild: bool,
    ) -> (Vec<ReducedChannel>, ReductionStats) {
        let values: Vec<_> = (start..end)
            .map(|i| {
                if i % 31 == 0 {
                    5.0
                } else {
                    ((i * 17) % 23) as f32
                }
            })
            .collect();
        let missing: Vec<_> = (start..end)
            .filter(|i| (i / 7) % 9 == 2)
            .map(|i| i..i + 1)
            .collect();
        let split = values.len() / 2;
        let views = [ChannelView {
            first: &values[..split],
            second: &values[split..],
            start,
            missing_first: &missing,
            missing_second: &[],
        }];
        let mut output = Vec::new();
        let stats = reducer.reduce(
            ReductionInput {
                channels: &views,
                buffered: start..end,
                viewport: start..start + 200,
                appended: previous..end,
                output_len: width,
                rebuild,
            },
            &mut output,
        );
        (output, stats)
    }

    #[test]
    fn incremental_gaps_tail_and_eviction_match_rebuilding_at_every_width() {
        for width in [1, 2, 9, 33, 400] {
            let mut reducer = LttbReducer::default();
            for end in 1u64..500 {
                let start = end.saturating_sub(200);
                let (output, stats) = run(&mut reducer, start, end, end - 1, width, end == 1);
                let (expected, _) =
                    run(&mut LttbReducer::default(), start, end, start, width, true);
                assert_eq!(output, expected, "width={width} end={end}");
                assert!(stats.visited_samples <= 1 + 200u64.div_ceil(width as u64) as usize);
                assert!(stats.cached_buckets <= width + 1);
                let ReducedChannel::Polyline(points) = &output[0] else {
                    panic!("polyline");
                };
                assert!(
                    points
                        .windows(2)
                        .all(|pair| pair[0].sample < pair[1].sample)
                );
                assert!(
                    points
                        .iter()
                        .all(|p| p.sample >= start && p.sample < end && (p.sample / 7) % 9 != 2)
                );
                if (end - 1) / 7 % 9 != 2 {
                    assert_eq!(points.last().unwrap().sample, end - 1);
                }
                for pair in points.windows(2) {
                    if !pair[1].break_before {
                        assert!((pair[0].sample..=pair[1].sample).all(|i| (i / 7) % 9 != 2));
                    }
                }
            }
        }
    }
}
