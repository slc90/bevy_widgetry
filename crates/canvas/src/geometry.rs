use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

pub(crate) fn fill(size: Vec2) -> Mesh {
    mesh(corners(size), vec![0, 1, 2, 0, 2, 3])
}

pub(crate) fn stroke(size: Vec2, width: f32) -> Mesh {
    let mut positions = corners(size + Vec2::splat(width));
    positions.extend(corners((size - Vec2::splat(width)).max(Vec2::ZERO)));
    ring(positions)
}

pub(crate) fn outline(points: [Vec2; 4], width: f32, gap: f32) -> Mesh {
    let winding = (points[1] - points[0])
        .perp_dot(points[2] - points[1])
        .signum();
    let normals = std::array::from_fn::<_, 4, _>(|i| {
        let edge = (points[(i + 1) % 4] - points[i]).normalize_or_zero();
        Vec2::new(edge.y, -edge.x) * winding
    });
    let mut positions = Vec::with_capacity(8);
    for distance in [gap + width * 0.5, gap - width * 0.5] {
        positions.extend((0..4).map(|i| {
            (points[i] + (normals[(i + 3) % 4] + normals[i]) * distance)
                .extend(0.0)
                .to_array()
        }));
    }
    ring(positions)
}

fn ring(positions: Vec<[f32; 3]>) -> Mesh {
    let mut indices = Vec::with_capacity(24);
    for i in 0..4 {
        let next = (i + 1) % 4;
        indices.extend([i, next, next + 4, i, next + 4, i + 4]);
    }
    mesh(positions, indices)
}

fn corners(size: Vec2) -> Vec<[f32; 3]> {
    let half = size * 0.5;
    vec![
        [-half.x, -half.y, 0.0],
        [half.x, -half.y, 0.0],
        [half.x, half.y, 0.0],
        [-half.x, half.y, 0.0],
    ]
}

fn mesh(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices))
}
