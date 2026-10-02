use crate::{ReducedChannel, WaveformRuntime, WaveformStyle};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

/// 创建带稳定 attribute layout 的 triangle mesh，后续更新复用 Vec allocation。
pub(crate) fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new())
    .with_inserted_indices(Indices::U32(Vec::new()))
}

/// 固定 range 使用 f64 换算，避免有限 f32 两端相减溢出；超出 range 的 sample clamp 到 lane。
fn lane_y(value: f32, min: f32, max: f32, channel: usize, channels: usize, height: f32) -> f32 {
    let normalized = ((f64::from(value) - f64::from(min)) / (f64::from(max) - f64::from(min)))
        .clamp(0.0, 1.0) as f32;
    height * (1.0 - (channel as f32 + 1.0 - normalized) / channels as f32)
}

/// 两点线段统一扩为 quad；水平/垂直 line 都使用 physical pixel 宽度。
fn quad(a: Vec2, b: Vec2, line_width: f32) -> [Vec2; 4] {
    let direction = b - a;
    let normal = if direction.length_squared() > 0.0 {
        Vec2::new(-direction.y, direction.x).normalize() * line_width * 0.5
    } else {
        Vec2::X * line_width * 0.5
    };
    [a - normal, a + normal, b + normal, b - normal]
}

/// 合并全部 channel；坐标归一化到固定正交 camera，resize 不依赖前一帧 projection。
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
    let Some(VertexAttributeValues::Float32x2(mut uvs)) =
        mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0)
    else {
        return Err(BevyError::error("Waveform Mesh requires uvs"));
    };
    let Some(Indices::U32(mut indices)) = mesh.remove_indices() else {
        return Err(BevyError::error("Waveform Mesh requires u32 indices"));
    };
    positions.clear();
    colors.clear();
    uvs.clear();
    indices.clear();
    let range = runtime.viewport_range();
    let capacity = (range.end - range.start) as f64;
    let sample_x = |sample: u64, fractional: f64| {
        (((sample - range.start) as f64 + fractional) / capacity) as f32 * size.x
    };
    let channels = runtime.config().channel_ranges.len();
    for (channel, reduced) in runtime.reduced_channels().iter().enumerate() {
        let value_range = &runtime.config().channel_ranges[channel];
        let y = |value| {
            lane_y(
                value,
                *value_range.start(),
                *value_range.end(),
                channel,
                channels,
                size.y,
            )
        };
        let color = style.palette[channel % style.palette.len()]
            .to_linear()
            .to_f32_array();
        let mut append = |points: [Vec2; 4]| {
            let base = positions.len() as u32;
            let bottom = size.y * (1.0 - (channel + 1) as f32 / channels as f32);
            let top = size.y * (1.0 - channel as f32 / channels as f32);
            positions.extend(points.map(|point| {
                [
                    point.x.clamp(0.0, size.x) / size.x - 0.5,
                    point.y.clamp(bottom, top) / size.y - 0.5,
                    0.0,
                ]
            }));
            colors.extend([color; 4]);
            uvs.extend([[0.0, 0.0]; 4]);
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
                    append(quad(
                        Vec2::new(x, middle - half),
                        Vec2::new(x, middle + half),
                        style.line_width,
                    ));
                }
            }
            ReducedChannel::Polyline(points) => {
                if points.len() == 1 {
                    let point = Vec2::new(sample_x(points[0].sample, 0.0), y(points[0].value));
                    append(quad(
                        point - Vec2::Y * style.line_width * 0.5,
                        point + Vec2::Y * style.line_width * 0.5,
                        style.line_width,
                    ));
                }
                for pair in points.windows(2) {
                    append(quad(
                        Vec2::new(sample_x(pair[0].sample, 0.0), y(pair[0].value)),
                        Vec2::new(sample_x(pair[1].sample, 0.0), y(pair[1].value)),
                        style.line_width,
                    ));
                }
            }
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    Ok(())
}

#[cfg(test)]
// geometry 测试通过断言验证数学 contract，生产代码不使用主动 panic。
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    // finite f32 极端端点仍正确映射 max/top 与 min/bottom；64 lane 等高且无 gap。
    #[test]
    fn fixed_ranges_map_to_equal_lanes_without_overflow() {
        for channel in 0..64 {
            assert_eq!(
                lane_y(f32::MAX, -f32::MAX, f32::MAX, channel, 64, 640.0),
                640.0 - channel as f32 * 10.0
            );
            assert_eq!(
                lane_y(-f32::MAX, -f32::MAX, f32::MAX, channel, 64, 640.0),
                630.0 - channel as f32 * 10.0
            );
        }
        let points = quad(Vec2::new(2.0, 3.0), Vec2::new(2.0, 7.0), 1.0);
        assert_eq!((points[0].x - points[1].x).abs(), 1.0);
    }
}
