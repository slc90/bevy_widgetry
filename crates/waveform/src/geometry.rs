use crate::{ReducedChannel, WaveformRuntime, WaveformStyle};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

pub(crate) fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    // Bevy mesh allocator 不接受零长度 allocation；零面积透明 triangle 不产生可见像素。
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0; 3]; 3])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0.0; 4]; 3])
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
    let channels = runtime.config().channel_ranges.len();
    for (channel, reduced) in runtime.reduced_channels().iter().enumerate() {
        let value_range = &runtime.config().channel_ranges[channel];
        let lane = LaneProjection::new(
            *value_range.start(),
            *value_range.end(),
            channel,
            channels,
            size.y,
        );
        let y = |value| lane.y(value);
        let color = style.palette[channel % style.palette.len()]
            .to_linear()
            .to_f32_array();
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
                if points.len() == 1 {
                    let point = Vec2::new(sample_x(points[0].sample, 0.0), y(points[0].value));
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
        indices.extend([0, 1, 2]);
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    Ok(())
}

#[cfg(test)]
// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

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
