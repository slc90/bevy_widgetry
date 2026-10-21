use crate::{ReducedChannel, WaveformRuntime, WaveformStyle};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_resource::VertexFormat;

const CHANNEL_ATTRIBUTE: MeshVertexAttribute =
    MeshVertexAttribute::new("WidgetryWaveformChannel", 0x5752_0001, VertexFormat::Uint32);
use bevy::math::DVec2;
use bevy::prelude::*;

pub(crate) fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    // The Bevy mesh allocator rejects zero-length allocations.
    // A transparent triangle with zero area satisfies allocation requirements without producing visible pixels.
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0; 3]; 3])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0; 4]; 3])
    .with_inserted_attribute(CHANNEL_ATTRIBUTE, vec![0_u32; 3])
    .with_inserted_indices(Indices::U32(vec![0, 1, 2]))
}

struct LaneProjection {
    center: f64,
    scale: f64,
}

impl LaneProjection {
    fn new(amplitude: crate::WaveformAmplitude, channel: usize, lanes: Vec2, height: f32) -> Self {
        let lane_height = f64::from(lanes.x);
        Self {
            center: f64::from(height) + f64::from(lanes.y) - (channel as f64 + 0.5) * lane_height,
            scale: lane_height / (2.0 * amplitude.volts()),
        }
    }
    fn y(&self, value: f32) -> f64 {
        self.center + f64::from(value) * self.scale
    }
}

fn clip_segment(mut a: DVec2, mut b: DVec2, size: Vec2, margin: f32) -> Option<(Vec2, Vec2)> {
    let low = DVec2::splat(-f64::from(margin));
    let high = size.as_dvec2() + DVec2::splat(f64::from(margin));
    // Clip before converting to f32 so extreme valid samples cannot overflow geometry.
    for axis in 0..2 {
        for (bound, direction) in [(low[axis], 1.0), (high[axis], -1.0)] {
            let da = (a[axis] - bound) * direction;
            let db = (b[axis] - bound) * direction;
            if da < 0.0 && db < 0.0 {
                return None;
            }
            if (da < 0.0) != (db < 0.0) {
                let mut intersection = a.lerp(b, da / (da - db));
                intersection[axis] = bound;
                if da < 0.0 {
                    a = intersection;
                } else {
                    b = intersection;
                }
            }
        }
    }
    Some((a.as_vec2(), b.as_vec2()))
}

fn clip_polygon(points: [Vec2; 4], size: Vec2) -> Vec<Vec2> {
    let mut polygon = points.to_vec();
    for axis in 0..2 {
        for (bound, direction) in [(0.0, 1.0), (size[axis], -1.0)] {
            let input = std::mem::take(&mut polygon);
            if input.is_empty() {
                return polygon;
            }
            let mut previous = input[input.len() - 1];
            for current in input {
                let da = (previous[axis] - bound) * direction;
                let db = (current[axis] - bound) * direction;
                if (da < 0.0) != (db < 0.0) {
                    let mut intersection = previous.lerp(current, da / (da - db));
                    intersection[axis] = bound;
                    polygon.push(intersection);
                }
                if db >= 0.0 {
                    polygon.push(current);
                }
                previous = current;
            }
        }
    }
    polygon
}

fn quad(a: Vec2, b: Vec2, line_width: f32) -> [Vec2; 4] {
    let direction = b - a;
    let normal = if direction.length_squared() > 0.0 {
        Vec2::new(-direction.y, direction.x).normalize() * line_width * 0.5
    } else {
        Vec2::X * line_width * 0.5
    };
    [a - normal, a + normal, b + normal, b - normal]
}

pub(crate) fn update_mesh(
    mesh: &mut Mesh,
    runtime: &WaveformRuntime,
    style: &WaveformStyle,
    size: Vec2,
    lanes: Vec2,
    palette: &[Color],
) -> Result<(), BevyError> {
    let Some(VertexAttributeValues::Float32x3(mut positions)) =
        mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err(BevyError::error("Waveform Mesh requires positions"));
    };
    let Some(VertexAttributeValues::Float32x4(mut colors)) =
        mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        return Err(BevyError::error("Waveform Mesh requires colors"));
    };
    let Some(Indices::U32(mut indices)) = mesh.remove_indices() else {
        return Err(BevyError::error("Waveform Mesh requires u32 indices"));
    };
    let Some(VertexAttributeValues::Uint32(mut vertex_channels)) =
        mesh.remove_attribute(CHANNEL_ATTRIBUTE)
    else {
        return Err(BevyError::error("Waveform Mesh requires channel metadata"));
    };
    vertex_channels.clear();
    positions.clear();
    colors.clear();
    indices.clear();
    let range = runtime.viewport_range();
    let capacity = (range.end - range.start) as f64;
    let inverse_size = size.recip();
    let sample_scale = f64::from(size.x) / capacity;
    let sample_x = |sample: u64, fractional: f64| {
        (((sample - range.start) as f64 + fractional) * sample_scale) as f32
    };
    for (channel, reduced) in runtime.reduced_channels().iter().enumerate() {
        let lane = LaneProjection::new(runtime.config().amplitude_range, channel, lanes, size.y);
        let y = |value| lane.y(value);
        let color = palette[channel % palette.len()].to_linear().to_f32_array();
        let mut append = |points: [Vec2; 4]| {
            let clipped;
            let polygon: &[Vec2] = if points
                .iter()
                .all(|p| p.x >= 0.0 && p.x <= size.x && p.y >= 0.0 && p.y <= size.y)
            {
                &points
            } else {
                clipped = clip_polygon(points, size);
                &clipped
            };
            if polygon.len() < 3 {
                return;
            }
            let base = positions.len() as u32;
            positions.extend(
                polygon
                    .iter()
                    .map(|p| [p.x * inverse_size.x - 0.5, p.y * inverse_size.y - 0.5, 0.0]),
            );
            colors.extend(std::iter::repeat_n(color, polygon.len()));
            vertex_channels.extend(std::iter::repeat_n(channel as u32, polygon.len()));
            for i in 1..polygon.len() as u32 - 1 {
                indices.extend([base, base + i, base + i + 1]);
            }
        };
        match reduced {
            ReducedChannel::Envelope(spans) => {
                for span in spans {
                    let x = sample_x(
                        span.samples.start,
                        (span.samples.end - span.samples.start) as f64 * 0.5,
                    );
                    let min = y(span.min);
                    let max = y(span.max);
                    let middle = (min + max) * 0.5;
                    let half = ((max - min) * 0.5).max(f64::from(style.line_width) * 0.5);
                    let low = (middle - half).max(0.0);
                    let high = (middle + half).min(f64::from(size.y));
                    if low >= high {
                        continue;
                    }
                    let left = x - style.line_width * 0.5;
                    let right = x + style.line_width * 0.5;
                    append([
                        Vec2::new(right, low as f32),
                        Vec2::new(left, low as f32),
                        Vec2::new(left, high as f32),
                        Vec2::new(right, high as f32),
                    ]);
                }
            }
            ReducedChannel::Polyline(points) => {
                for (index, sample) in points.iter().enumerate() {
                    let before = index > 0 && !sample.break_before;
                    let after = points.get(index + 1).is_some_and(|next| !next.break_before);
                    if before || after {
                        continue;
                    }
                    let point =
                        DVec2::new(f64::from(sample_x(sample.sample, 0.0)), y(sample.value));
                    if let Some((a, b)) = clip_segment(
                        point - DVec2::Y * f64::from(style.line_width) * 0.5,
                        point + DVec2::Y * f64::from(style.line_width) * 0.5,
                        size,
                        style.line_width,
                    ) {
                        append(quad(a, b, style.line_width));
                    }
                }
                for pair in points.windows(2) {
                    if pair[1].break_before {
                        continue;
                    }
                    if let Some((a, b)) = clip_segment(
                        DVec2::new(f64::from(sample_x(pair[0].sample, 0.0)), y(pair[0].value)),
                        DVec2::new(f64::from(sample_x(pair[1].sample, 0.0)), y(pair[1].value)),
                        size,
                        style.line_width,
                    ) {
                        append(quad(a, b, style.line_width));
                    }
                }
            }
        }
    }
    if positions.is_empty() {
        positions.resize(3, [0.0; 3]);
        colors.resize(3, [0.0; 4]);
        vertex_channels.resize(3, 0);
        indices.extend([0, 1, 2]);
    }
    mesh.insert_attribute(CHANNEL_ATTRIBUTE, vertex_channels);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    Ok(())
}

pub(crate) fn recolor_mesh(mesh: &mut Mesh, palette: &[Color]) -> Result<(), BevyError> {
    let Some(VertexAttributeValues::Uint32(channels)) = mesh.attribute(CHANNEL_ATTRIBUTE) else {
        return Err(BevyError::error("Waveform Mesh requires channel metadata"));
    };
    let colors = channels
        .iter()
        .map(|channel| {
            palette[*channel as usize % palette.len()]
                .to_linear()
                .to_f32_array()
        })
        .collect::<Vec<_>>();
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn sparse_points_from_custom_reducer_remain_connected() {
        struct Sparse;
        impl crate::WaveformReducer for Sparse {
            fn reduce(
                &mut self,
                _: crate::ReductionInput<'_>,
                output: &mut Vec<ReducedChannel>,
            ) -> crate::ReductionStats {
                output.clear();
                output.push(ReducedChannel::Polyline(
                    [0, 10, 20]
                        .map(|sample| crate::WaveformPoint {
                            sample,
                            value: 0.5,
                            break_before: false,
                        })
                        .to_vec(),
                ));
                crate::ReductionStats::default()
            }
        }
        struct Source;
        impl crate::WaveformSource for Source {
            fn metadata(&self) -> Result<crate::WaveformMetadata, crate::WaveformReadError> {
                Ok(crate::WaveformMetadata {
                    channel_count: 1,
                    channel_labels: vec!["A".into()],
                    sample_rate: 1000,
                    unit: crate::WaveformUnit::Volts,
                    kind: crate::WaveformSessionKind::Live,
                })
            }
            fn read(
                &self,
                request: &crate::WaveformReadRequest,
                out: &mut crate::PlanarBuffer,
            ) -> Result<crate::WaveformReadStatus, crate::WaveformReadError> {
                out.channels_mut()[0]
                    .resize((request.range.end - request.range.start) as usize, 0.5);
                Ok(crate::WaveformReadStatus::Ready(request.clone()))
            }
        }
        let mut runtime = WaveformRuntime::with_reducer(
            crate::WaveformConfig {
                visible_duration_ms: 21,
                ..default()
            },
            std::sync::Arc::new(Source),
            Box::new(Sparse),
        )
        .unwrap();
        runtime
            .update(std::time::Duration::from_millis(21), 3)
            .unwrap();
        let mut mesh = empty_mesh();
        update_mesh(
            &mut mesh,
            &runtime,
            &WaveformStyle::default(),
            Vec2::new(100.0, 100.0),
            Vec2::new(100.0, 0.0),
            &[Color::WHITE],
        )
        .unwrap();
        assert_eq!(mesh.count_vertices(), 8);
    }

    #[test]
    fn missing_samples_do_not_create_bridges_and_isolated_samples_remain_visible() {
        use crate::{
            PlanarBuffer, WaveformConfig, WaveformMetadata, WaveformReadError, WaveformReadRequest,
            WaveformReadStatus, WaveformSessionKind, WaveformSource, WaveformUnit,
        };
        struct Source(std::ops::Range<usize>);
        impl WaveformSource for Source {
            fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
                Ok(WaveformMetadata {
                    channel_count: 1,
                    channel_labels: vec!["A".into()],
                    sample_rate: 1000,
                    unit: WaveformUnit::Volts,
                    kind: WaveformSessionKind::Live,
                })
            }
            fn read(
                &self,
                request: &WaveformReadRequest,
                out: &mut PlanarBuffer,
            ) -> Result<WaveformReadStatus, WaveformReadError> {
                out.channels_mut()[0]
                    .resize((request.range.end - request.range.start) as usize, 0.5);
                out.missing_mut()[0].push(self.0.clone());
                Ok(WaveformReadStatus::Ready(request.clone()))
            }
        }
        for (gap, vertices) in [(4..6, 24), (1..9, 8), (0..10, 3)] {
            let mut runtime = WaveformRuntime::new(
                WaveformConfig {
                    visible_duration_ms: 10,
                    ..default()
                },
                std::sync::Arc::new(Source(gap)),
            )
            .unwrap();
            runtime
                .update(std::time::Duration::from_millis(10), 100)
                .unwrap();
            let mut mesh = empty_mesh();
            update_mesh(
                &mut mesh,
                &runtime,
                &WaveformStyle::default(),
                Vec2::new(100.0, 100.0),
                Vec2::new(100.0, 0.0),
                &[Color::WHITE],
            )
            .unwrap();
            assert_eq!(mesh.count_vertices(), vertices);
        }
    }

    #[test]
    fn palette_color_updates_preserve_geometry() {
        let mut mesh = empty_mesh();
        mesh.insert_attribute(CHANNEL_ATTRIBUTE, vec![0_u32, 1, 0]);
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap().clone();
        let indices = mesh.indices().unwrap().clone();
        recolor_mesh(&mut mesh, &[Color::BLACK, Color::WHITE]).unwrap();
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap(),
            &positions
        );
        assert_eq!(mesh.indices().unwrap(), &indices);
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap(),
            &VertexAttributeValues::Float32x4(vec![
                [0.0, 0.0, 0.0, 1.0],
                [1.0, 1.0, 1.0, 1.0],
                [0.0, 0.0, 0.0, 1.0]
            ])
        );
    }

    #[test]
    fn symmetric_scale_allows_cross_lane_excursions_and_matches_row_height() {
        let amplitude = crate::WaveformAmplitude::default();
        let lane = LaneProjection::new(amplitude, 1, Vec2::new(100.0, 0.0), 400.0);
        assert_eq!(lane.y(0.0), 250.0);
        assert_eq!(lane.y(1.0), 300.0);
        assert_eq!(lane.y(3.0), 400.0);
        assert_eq!(lane.y(-3.0), 100.0);
        assert_eq!(
            LaneProjection::new(amplitude, 1, Vec2::new(200.0, 0.0), 800.0).y(3.0),
            800.0
        );
        let points = quad(Vec2::new(2.0, 3.0), Vec2::new(2.0, 7.0), 1.0);
        assert_eq!((points[0].x - points[1].x).abs(), 1.0);
    }

    #[test]
    fn clipping_omits_outside_segments_and_keeps_true_reentry_intersection() {
        let size = Vec2::splat(100.0);
        assert!(
            clip_segment(DVec2::new(0.0, 200.0), DVec2::new(100.0, 200.0), size, 0.0).is_none()
        );
        let (a, b) =
            clip_segment(DVec2::new(0.0, 200.0), DVec2::new(100.0, 0.0), size, 0.0).unwrap();
        assert_eq!(a, Vec2::new(50.0, 100.0));
        assert_eq!(b, Vec2::new(100.0, 0.0));
        for point in clip_polygon(quad(a, b, 2.0), size) {
            assert!(point.x >= 0.0 && point.x <= 100.0 && point.y >= 0.0 && point.y <= 100.0);
        }
        let (a, b) =
            clip_segment(DVec2::new(0.0, 1e80), DVec2::new(100.0, -1e80), size, 0.0).unwrap();
        assert_eq!(a.y, 100.0);
        assert_eq!(b.y, 0.0);
    }

    #[test]
    fn envelopes_and_polylines_keep_neighbor_lane_shapes_and_skip_outside_runs() {
        struct Source;
        impl crate::WaveformSource for Source {
            fn metadata(&self) -> Result<crate::WaveformMetadata, crate::WaveformReadError> {
                Ok(crate::WaveformMetadata {
                    channel_count: 4,
                    channel_labels: vec!["A".into(); 4],
                    sample_rate: 1000,
                    unit: crate::WaveformUnit::Volts,
                    kind: crate::WaveformSessionKind::Live,
                })
            }
            fn read(
                &self,
                request: &crate::WaveformReadRequest,
                _: &mut crate::PlanarBuffer,
            ) -> Result<crate::WaveformReadStatus, crate::WaveformReadError> {
                Ok(crate::WaveformReadStatus::Ready(request.clone()))
            }
        }
        struct Reduced(Vec<ReducedChannel>);
        impl crate::WaveformReducer for Reduced {
            fn reduce(
                &mut self,
                _: crate::ReductionInput<'_>,
                output: &mut Vec<ReducedChannel>,
            ) -> crate::ReductionStats {
                output.clone_from(&self.0);
                default()
            }
        }
        for envelope in [false, true] {
            let reduced = (0..4)
                .map(|_| {
                    if envelope {
                        ReducedChannel::Envelope(vec![crate::WaveformSpan {
                            samples: crate::RangeBoundary {
                                start: 0,
                                end: 1000,
                            },
                            min: 3.0,
                            max: 3.0,
                        }])
                    } else {
                        ReducedChannel::Polyline(vec![
                            crate::WaveformPoint {
                                sample: 100,
                                value: 3.0,
                                break_before: true,
                            },
                            crate::WaveformPoint {
                                sample: 900,
                                value: 3.0,
                                break_before: false,
                            },
                        ])
                    }
                })
                .collect();
            let mut runtime = WaveformRuntime::with_reducer(
                default(),
                std::sync::Arc::new(Source),
                Box::new(Reduced(reduced)),
            )
            .unwrap();
            runtime.update(std::time::Duration::ZERO, 1000).unwrap();
            let mut mesh = empty_mesh();
            update_mesh(
                &mut mesh,
                &runtime,
                &WaveformStyle::default(),
                Vec2::splat(400.0),
                Vec2::new(100.0, 0.0),
                &[Color::WHITE],
            )
            .unwrap();
            let Some(VertexAttributeValues::Uint32(channels)) = mesh.attribute(CHANNEL_ATTRIBUTE)
            else {
                panic!("missing channel metadata")
            };
            assert!(
                !channels.contains(&0),
                "a wholly outside run must not form a boundary plateau"
            );
            assert!(channels.contains(&1) && channels.contains(&2) && channels.contains(&3));
            let Some(VertexAttributeValues::Float32x3(points)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("missing positions")
            };
            let lane_two: Vec<_> = channels
                .iter()
                .zip(points)
                .filter(|(channel, _)| **channel == 2)
                .map(|(_, p)| p[1])
                .collect();
            assert!(
                lane_two.iter().all(|y| (*y - 0.25).abs() < 0.01),
                "+3A belongs above the original lane"
            );
        }
    }
}
