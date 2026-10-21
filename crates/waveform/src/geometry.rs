use crate::{ReducedChannel, WaveformRuntime, WaveformStyle};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, MeshVertexAttribute, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_resource::VertexFormat;

const CHANNEL_ATTRIBUTE: MeshVertexAttribute =
    MeshVertexAttribute::new("WidgetryWaveformChannel", 0x5752_0001, VertexFormat::Uint32);
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
    min: f64,
    inverse_range: f64,
    bottom: f32,
    top: f32,
    height: f32,
    bottom_clip: f32,
    top_clip: f32,
}

impl LaneProjection {
    fn new(min: f32, max: f32, channel: usize, channels: usize, height: f32) -> Self {
        let bottom_clip = 0.5 - (channel + 1) as f32 / channels as f32;
        let top_clip = 0.5 - channel as f32 / channels as f32;
        Self {
            min: f64::from(min),
            inverse_range: 1.0 / (f64::from(max) - f64::from(min)),
            bottom: height * (bottom_clip + 0.5),
            top: height * (top_clip + 0.5),
            height: height / channels as f32,
            bottom_clip,
            top_clip,
        }
    }
    fn y(&self, value: f32) -> f32 {
        self.bottom
            + ((f64::from(value) - self.min) * self.inverse_range).clamp(0.0, 1.0) as f32
                * self.height
    }
    fn clip(&self, y: f32, inverse_height: f32) -> f32 {
        if y <= self.bottom {
            self.bottom_clip
        } else if y >= self.top {
            self.top_clip
        } else {
            (y * inverse_height - 0.5).clamp(self.bottom_clip, self.top_clip)
        }
    }
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
    let channels = runtime.channel_count();
    for (channel, reduced) in runtime.reduced_channels().iter().enumerate() {
        let value_range = &runtime.config().amplitude_range;
        let lane = LaneProjection::new(
            *value_range.start(),
            *value_range.end(),
            channel,
            channels,
            size.y,
        );
        let y = |value| lane.y(value);
        let color = palette[channel % palette.len()].to_linear().to_f32_array();
        let normalize = |point: Vec2| {
            [
                point.x.clamp(0.0, size.x) * inverse_size.x - 0.5,
                lane.clip(point.y, inverse_size.y),
                0.0,
            ]
        };
        let mut append = |points: [[f32; 3]; 4]| {
            let base = positions.len() as u32;
            positions.extend(points);
            colors.extend([color; 4]);
            vertex_channels.extend([channel as u32; 4]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
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
                    let half = ((max - min) * 0.5).max(style.line_width * 0.5);
                    let left =
                        (x - style.line_width * 0.5).clamp(0.0, size.x) * inverse_size.x - 0.5;
                    let right =
                        (x + style.line_width * 0.5).clamp(0.0, size.x) * inverse_size.x - 0.5;
                    let low = lane.clip(middle - half, inverse_size.y);
                    let high = lane.clip(middle + half, inverse_size.y);
                    append([
                        [right, low, 0.0],
                        [left, low, 0.0],
                        [left, high, 0.0],
                        [right, high, 0.0],
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
                    let point = Vec2::new(sample_x(sample.sample, 0.0), y(sample.value));
                    append(
                        quad(
                            point - Vec2::Y * style.line_width * 0.5,
                            point + Vec2::Y * style.line_width * 0.5,
                            style.line_width,
                        )
                        .map(normalize),
                    );
                }
                for pair in points.windows(2) {
                    if pair[1].break_before {
                        continue;
                    }
                    append(
                        quad(
                            Vec2::new(sample_x(pair[0].sample, 0.0), y(pair[0].value)),
                            Vec2::new(sample_x(pair[1].sample, 0.0), y(pair[1].value)),
                            style.line_width,
                        )
                        .map(normalize),
                    );
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
    fn fixed_ranges_map_to_equal_lanes_without_overflow() {
        for channel in 0..64 {
            assert_eq!(
                LaneProjection::new(-f32::MAX, f32::MAX, channel, 64, 640.0).y(f32::MAX),
                640.0 - channel as f32 * 10.0
            );
            assert_eq!(
                LaneProjection::new(-f32::MAX, f32::MAX, channel, 64, 640.0).y(-f32::MAX),
                630.0 - channel as f32 * 10.0
            );
        }
        let points = quad(Vec2::new(2.0, 3.0), Vec2::new(2.0, 7.0), 1.0);
        assert_eq!((points[0].x - points[1].x).abs(), 1.0);
    }
}
