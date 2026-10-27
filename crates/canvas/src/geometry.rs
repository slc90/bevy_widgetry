use crate::{CanvasGeometry, CanvasPaint};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

pub(crate) fn meshes(geometry: &CanvasGeometry, paint: CanvasPaint) -> Result<(Mesh, Mesh)> {
    let width = paint.stroke.map_or(0.0, |stroke| stroke.width);
    match geometry {
        CanvasGeometry::Rectangle(rectangle) => {
            if paint
                .stroke
                .is_some_and(|s| s.join != crate::CanvasLineJoin::Miter)
            {
                use crate::CanvasPathCommand::*;
                let half = rectangle.size * 0.5;
                crate::path::meshes(
                    &crate::CanvasPath {
                        commands: vec![
                            MoveTo(-half),
                            LineTo(Vec2::new(half.x, -half.y)),
                            LineTo(half),
                            LineTo(Vec2::new(-half.x, half.y)),
                            Close,
                        ],
                    },
                    paint,
                )
            } else {
                Ok((fill(rectangle.size), stroke(rectangle.size, width)))
            }
        }
        CanvasGeometry::Path(path) => crate::path::meshes(path, paint),
        CanvasGeometry::Ellipse(ellipse) => {
            use lyon::tessellation::*;
            // Tessellate in a bounded coordinate range: lyon merges very small points and squares lengths.
            // Keeping tolerance relative also makes uniform local/model scaling preserve the same geometry.
            let tolerance = 1.0 / 4096.0;
            let mut filled: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
            FillTessellator::new()
                .tessellate_path(
                    &ellipse_path(Vec2::ONE),
                    &FillOptions::default()
                        .with_fill_rule(match paint.fill_rule {
                            crate::CanvasFillRule::NonZero => FillRule::NonZero,
                            crate::CanvasFillRule::EvenOdd => FillRule::EvenOdd,
                        })
                        .with_tolerance(tolerance),
                    &mut BuffersBuilder::new(&mut filled, |vertex: FillVertex| {
                        vertex.position().to_array()
                    }),
                )
                .map_err(BevyError::error)?;
            let mut stroked: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
            let stroke_scale = ellipse.radii + Vec2::splat(width * 0.5);
            if width > 0.0 {
                let path = ellipse_stroke_path(ellipse.radii, width, stroke_scale, tolerance);
                FillTessellator::new()
                    .tessellate_path(
                        &path,
                        &FillOptions::default()
                            .with_fill_rule(FillRule::NonZero)
                            .with_tolerance(tolerance),
                        &mut BuffersBuilder::new(&mut stroked, |vertex: FillVertex| {
                            vertex.position().to_array()
                        }),
                    )
                    .map_err(BevyError::error)?;
            }
            let convert = |vertices: Vec<[f32; 2]>, scale: Vec2| -> Result<Vec<[f32; 3]>> {
                vertices
                    .into_iter()
                    .map(|p| {
                        let position = Vec2::from_array(p) * scale;
                        if !position.is_finite() {
                            return Err(BevyError::error(
                                "Canvas tessellated ellipse bounds overflow",
                            ));
                        }
                        Ok([position.x, position.y, 0.0])
                    })
                    .collect()
            };
            Ok((
                mesh(convert(filled.vertices, ellipse.radii)?, filled.indices),
                mesh(convert(stroked.vertices, stroke_scale)?, stroked.indices),
            ))
        }
    }
}

fn ellipse_stroke_path(radii: Vec2, width: f32, scale: Vec2, tolerance: f32) -> lyon::path::Path {
    use bevy::math::DVec2;
    use lyon::math::point;
    let solid = width >= radii.min_element() * 2.0;
    let radii = radii.as_dvec2();
    let scale = scale.as_dvec2();
    let half_width = f64::from(width) * 0.5;
    let position = |direction: DVec2| {
        (direction * radii + (direction / radii).normalize() * half_width) / scale
    };
    let mut directions = vec![DVec2::X];
    let axes = [DVec2::X, DVec2::Y, -DVec2::X, -DVec2::Y, DVec2::X];
    for pair in axes.windows(2) {
        let mut pending = vec![(pair[0], pair[1], position(pair[0]), position(pair[1]))];
        while let Some((from, to, a, b)) = pending.pop() {
            let chord = b - a;
            let normal = DVec2::new(chord.y, -chord.x).normalize_or_zero();
            let local_normal = normal / scale;
            // The support of ellipse + disk gives the maximum distance from this chord,
            // including sharp normal changes near the ends of very eccentric ellipses.
            let support = (radii * local_normal).length() + half_width * local_normal.length();
            if support - normal.dot(a) <= f64::from(tolerance) {
                directions.push(to);
            } else {
                // Bisect vectors rather than angles to retain tiny directions near every axis.
                let middle = (from + to).normalize();
                let p = position(middle);
                pending.push((middle, to, p, b));
                pending.push((from, middle, a, p));
            }
        }
    }
    let mut path = lyon::path::Path::builder();
    let point = |p: DVec2| point(p.x as f32, p.y as f32);
    if solid {
        // Once erosion removes the inner hole, the stroke is the convex outer offset.
        path.begin(point(position(directions[0])));
        for direction in directions.into_iter().skip(1) {
            path.line_to(point(position(direction)));
        }
        path.end(true);
    } else {
        let inner = |direction: DVec2| {
            (direction * radii - (direction / radii).normalize() * half_width) / scale
        };
        for pair in directions.windows(2) {
            let a = position(pair[0]);
            let b = position(pair[1]);
            let c = inner(pair[1]);
            let d = inner(pair[0]);
            // The inward offsets may cross near sharp ends. Union consistently wound
            // ribbon triangles so every part of the stroke receives paint exactly once.
            for [a, mut b, mut c] in [[a, b, c], [a, c, d]] {
                let area = (b - a).perp_dot(c - a);
                if area == 0.0 {
                    continue;
                }
                if area < 0.0 {
                    std::mem::swap(&mut b, &mut c);
                }
                path.begin(point(a));
                path.line_to(point(b));
                path.line_to(point(c));
                path.end(true);
            }
        }
    }
    path.build()
}

fn ellipse_path(radii: Vec2) -> lyon::path::Path {
    use lyon::math::point;
    let mut builder = lyon::path::Path::builder();
    builder.begin(point(radii.x, 0.0));
    // The builder's quadratic ellipse has a fixed relative error independent of tessellation tolerance.
    // Eight cubic arcs keep that error below the relative tessellation floor.
    let step = std::f32::consts::FRAC_PI_4;
    let k = (step * 0.25).tan() * (4.0 / 3.0);
    for index in 0..8 {
        let from = Vec2::from_angle(index as f32 * step);
        let to = if index == 7 {
            Vec2::X
        } else {
            Vec2::from_angle((index + 1) as f32 * step)
        };
        let control_a = (from + from.perp() * k) * radii;
        let control_b = (to - to.perp() * k) * radii;
        let end = to * radii;
        builder.cubic_bezier_to(
            point(control_a.x, control_a.y),
            point(control_b.x, control_b.y),
            point(end.x, end.y),
        );
    }
    builder.end(true);
    builder.build()
}

pub(crate) fn fill(size: Vec2) -> Mesh {
    mesh(corners(size), vec![0, 1, 2, 0, 2, 3])
}

pub(crate) fn stroke(size: Vec2, width: f32) -> Mesh {
    let mut positions = corners(size + Vec2::splat(width));
    positions.extend(corners((size - Vec2::splat(width)).max(Vec2::ZERO)));
    ring(positions)
}

pub(crate) fn outline(
    points: [Vec2; 4],
    width: f32,
    gap: f32,
    controls: [Vec2; 8],
    handle_size: f32,
    rotation: Vec2,
    rotation_size: f32,
) -> Mesh {
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
    let mut indices = ring_indices();
    for center in controls {
        let base = positions.len() as u32;
        positions.extend(
            corners(Vec2::splat(handle_size))
                .into_iter()
                .map(|p| (center + Vec2::new(p[0], p[1])).extend(0.0).to_array()),
        );
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    let top = controls[1];
    let direction = (rotation - top).normalize_or_zero();
    let normal = Vec2::new(-direction.y, direction.x) * width * 0.25;
    let base = positions.len() as u32;
    positions.extend(
        [
            top - normal,
            rotation - normal,
            rotation + normal,
            top + normal,
        ]
        .map(|p| p.extend(0.0).to_array()),
    );
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    let base = positions.len() as u32;
    positions.push(rotation.extend(0.0).to_array());
    for i in 0..12 {
        let angle = i as f32 * std::f32::consts::TAU / 12.0;
        positions.push(
            (rotation + Vec2::new(angle.cos(), angle.sin()) * rotation_size * 0.5)
                .extend(0.0)
                .to_array(),
        );
        indices.extend([base, base + 1 + i, base + 1 + (i + 1) % 12]);
    }
    mesh(positions, indices)
}

fn ring(positions: Vec<[f32; 3]>) -> Mesh {
    mesh(positions, ring_indices())
}

fn ring_indices() -> Vec<u32> {
    let mut indices = Vec::with_capacity(24);
    for i in 0..4 {
        let next = (i + 1) % 4;
        indices.extend([i, next, next + 4, i, next + 4, i + 4]);
    }
    indices
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

pub(crate) fn mesh(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_indices(Indices::U32(indices))
}
