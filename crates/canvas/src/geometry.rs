use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

pub(crate) fn fill(size: Vec2) -> Mesh {
    mesh(corners(size), vec![0, 1, 2, 0, 2, 3])
}

pub(crate) fn stroke(size: Vec2, width: f32) -> Mesh {
    let mut positions = corners(size + Vec2::splat(width));
    positions.extend(corners((size - Vec2::splat(width)).max(Vec2::ZERO)));
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
