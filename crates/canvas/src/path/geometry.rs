use super::{CanvasPath, Segment};
use crate::{CanvasFillRule, CanvasLineCap, CanvasLineJoin, CanvasPaint};
use bevy::{math::DVec2, prelude::*};
use lyon::{math::point, tessellation::*};
use std::collections::BTreeMap;

struct Contour {
    points: Vec<DVec2>,
    closed: bool,
    has_segments: bool,
    min: DVec2,
    max: DVec2,
}

pub(crate) fn meshes(source: &CanvasPath, paint: CanvasPaint) -> Result<(Mesh, Mesh)> {
    let mut contours = Vec::new();
    let mut current = Vec::new();
    for segment in source.segments()? {
        if matches!(segment, Segment::Move(_)) && !current.is_empty() {
            contours.push(contour(std::mem::take(&mut current))?);
        }
        current.push(segment);
    }
    if !current.is_empty() {
        contours.push(contour(current)?);
    }
    let width = f64::from(paint.stroke.map_or(0.0, |s| s.width));
    let mut fill = VertexBuffers::<[f32; 3], u32>::new();
    let mut stroke = VertexBuffers::<[f32; 3], u32>::new();
    for group in groups(&contours, 0.0) {
        let polygons = group.iter().map(|i| contours[*i].points.clone()).collect();
        paint_regions(&mut fill, polygons, 0.0, &[], paint.fill_rule)?;
    }
    if width > 0.0 {
        for group in groups(&contours, width * 2.0) {
            let mut polygons = Vec::new();
            let mut center_edges = Vec::new();
            for index in group {
                let contour = &contours[index];
                if !contour.has_segments {
                    continue;
                }
                append_stroke(
                    &mut polygons,
                    &mut center_edges,
                    &contour.points,
                    contour.closed,
                    width * 0.5,
                    paint.stroke.map_or_default(|s| s.cap),
                    paint.stroke.map_or_default(|s| s.join),
                );
            }
            paint_regions(
                &mut stroke,
                polygons,
                width,
                &center_edges,
                CanvasFillRule::NonZero,
            )?;
        }
    }
    Ok((
        crate::geometry::mesh(fill.vertices, fill.indices),
        crate::geometry::mesh(stroke.vertices, stroke.indices),
    ))
}

fn contour(segments: Vec<Segment>) -> Result<Contour> {
    let has_segments = segments.iter().any(|s| !matches!(s, Segment::Move(_)));
    let mut points = Vec::new();
    let mut closed = false;
    let mut current = DVec2::ZERO;
    for segment in segments {
        match segment {
            Segment::Move(p) | Segment::Line(p) => {
                points.push(p);
                current = p;
            }
            Segment::Quad([c, to]) => {
                flatten_curve(&[current, c, to], &mut points);
                current = to;
            }
            Segment::Cubic([a, b, to]) => {
                flatten_curve(&[current, a, b, to], &mut points);
                current = to;
            }
            Segment::Close => {
                closed = true;
            }
        }
    }
    let min = points
        .iter()
        .copied()
        .fold(DVec2::splat(f64::INFINITY), DVec2::min);
    let max = points
        .iter()
        .copied()
        .fold(DVec2::splat(f64::NEG_INFINITY), DVec2::max);
    if !min.is_finite() || !max.is_finite() {
        return Err(BevyError::error("Canvas Path flattened bounds overflow"));
    }
    Ok(Contour {
        points,
        closed,
        has_segments,
        min,
        max,
    })
}

fn flatten_curve(controls: &[DVec2], output: &mut Vec<DVec2>) {
    let min = controls
        .iter()
        .copied()
        .fold(DVec2::splat(f64::INFINITY), DVec2::min);
    let max = controls
        .iter()
        .copied()
        .fold(DVec2::splat(f64::NEG_INFINITY), DVec2::max);
    let origin = (min + max) * 0.5;
    let scale = ((max - min) * 0.5).map(|v| if v == 0.0 { 1.0 } else { v });
    let p = |i: usize| {
        let p = (controls[i] - origin) / scale;
        point(p.x as f32, p.y as f32)
    };
    let mut append = |line: &lyon::geom::LineSegment<f32>| {
        output.push(origin + DVec2::new(f64::from(line.to.x), f64::from(line.to.y)) * scale);
    };
    if controls.len() == 3 {
        lyon::geom::QuadraticBezierSegment {
            from: p(0),
            ctrl: p(1),
            to: p(2),
        }
        .for_each_flattened(1.0 / 4096.0, &mut append);
    } else {
        lyon::geom::CubicBezierSegment {
            from: p(0),
            ctrl1: p(1),
            ctrl2: p(2),
            to: p(3),
        }
        .for_each_flattened(1.0 / 4096.0, &mut append);
    }
    if let (Some(last), Some(to)) = (output.last_mut(), controls.last()) {
        *last = *to;
    }
}

fn groups(contours: &[Contour], margin: f64) -> Vec<Vec<usize>> {
    fn root(parents: &mut [usize], mut index: usize) -> usize {
        while parents[index] != index {
            parents[index] = parents[parents[index]];
            index = parents[index];
        }
        index
    }
    let mut parents = (0..contours.len()).collect::<Vec<_>>();
    for (i, a) in contours.iter().enumerate() {
        for (j, b) in contours.iter().enumerate().skip(i + 1) {
            let gap = DVec2::splat(margin * 2.0);
            if (a.min - b.max).cmple(gap).all() && (b.min - a.max).cmple(gap).all() {
                let a = root(&mut parents, i);
                let b = root(&mut parents, j);
                parents[b] = a;
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for i in 0..contours.len() {
        groups.entry(root(&mut parents, i)).or_default().push(i);
    }
    groups.into_values().collect()
}

fn append_convex(target: &mut VertexBuffers<[f32; 3], u32>, polygon: &[DVec2]) -> Result {
    let mut vertices = VertexBuffers::new();
    vertices.vertices = polygon.iter().map(|p| [p.x as f32, p.y as f32]).collect();
    for i in 1..polygon.len().saturating_sub(1) {
        vertices.indices.extend([
            0,
            u32::try_from(i).map_err(BevyError::error)?,
            u32::try_from(i + 1).map_err(BevyError::error)?,
        ]);
    }
    append_mesh(target, vertices, DVec2::ZERO, DVec2::ONE)
}

fn paint_regions(
    target: &mut VertexBuffers<[f32; 3], u32>,
    polygons: Vec<Vec<DVec2>>,
    feature_floor: f64,
    center_edges: &[DVec2],
    rule: CanvasFillRule,
) -> Result {
    let coordinates: [Vec<f64>; 2] = std::array::from_fn(|axis| {
        let mut values = polygons
            .iter()
            .flatten()
            .map(|p| f64::from(p[axis] as f32))
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        values.dedup();
        values
    });
    let center_spacing = center_edges
        .iter()
        .map(|edge| edge.length())
        .fold(f64::INFINITY, f64::min);
    let mut pending = vec![polygons];
    while let Some(mut polygons) = pending.pop() {
        for p in &mut polygons {
            p.dedup();
            if p.len() > 1 && p.first() == p.last() {
                p.pop();
            }
        }
        polygons.retain(|p| {
            p.len() >= 3
                && !(p.len() == 3
                    && (p[0].as_vec2() == p[1].as_vec2()
                        || p[1].as_vec2() == p[2].as_vec2()
                        || p[0].as_vec2() == p[2].as_vec2()))
        });
        if polygons.is_empty() {
            continue;
        }
        if feature_floor / center_spacing > 4096.0
            && let Some(axis) = (0..2).find(|axis| {
                polygons.iter().flatten().any(|p| p[*axis] < 0.0)
                    && polygons.iter().flatten().any(|p| p[*axis] > 0.0)
            })
        {
            // Opposite distant vertices can also lose the short cap spacing during GPU clipping.
            pending.push(polygons.iter().map(|p| clip(p, axis, 0.0, false)).collect());
            pending.push(polygons.iter().map(|p| clip(p, axis, 0.0, true)).collect());
            continue;
        }
        if polygons.len() == 1 && convex(&polygons[0]) {
            let polygon = &polygons[0];
            let anchor = polygon[0].as_vec2().as_dvec2();
            let has_area = polygon[1..].windows(2).any(|p| {
                (p[0].as_vec2().as_dvec2() - anchor).perp_dot(p[1].as_vec2().as_dvec2() - anchor)
                    != 0.0
            });
            if !has_area {
                // A diagonal strip can collapse at both distant ends while crossing representable local coordinates.
                if let Some(axis) = (0..2).find(|axis| {
                    polygon.iter().any(|p| p[*axis] < 0.0) && polygon.iter().any(|p| p[*axis] > 0.0)
                }) {
                    pending.push(vec![clip(polygon, axis, 0.0, false)]);
                    pending.push(vec![clip(polygon, axis, 0.0, true)]);
                    continue;
                }
            }
            append_convex(target, &polygons[0])?;
            continue;
        }
        let mut min = DVec2::splat(f64::INFINITY);
        let mut max = DVec2::splat(f64::NEG_INFINITY);
        let mut feature = DVec2::splat(f64::INFINITY);
        for p in &polygons {
            let lo = p
                .iter()
                .copied()
                .fold(DVec2::splat(f64::INFINITY), DVec2::min);
            let hi = p
                .iter()
                .copied()
                .fold(DVec2::splat(f64::NEG_INFINITY), DVec2::max);
            min = min.min(lo);
            max = max.max(hi);
            for axis in 0..2 {
                let v = hi[axis] - lo[axis];
                if v > 0.0 {
                    feature[axis] = feature[axis].min(v);
                }
            }
        }
        // Original axis coordinates retain their precision budget across clipping.
        for axis in 0..2 {
            let values = &coordinates[axis];
            let begin = values.partition_point(|v| *v < f64::from(min[axis] as f32));
            let end = values.partition_point(|v| *v <= f64::from(max[axis] as f32));
            for pair in values[begin..end].windows(2) {
                feature[axis] = feature[axis].min(pair[1] - pair[0]);
            }
        }
        // Clipping intersections can reveal axis features absent from the source vertices.
        for axis in 0..2 {
            let mut values = polygons
                .iter()
                .flatten()
                .map(|p| f64::from(p[axis] as f32))
                .collect::<Vec<_>>();
            values.sort_by(f64::total_cmp);
            values.dedup();
            for pair in values.windows(2) {
                feature[axis] = feature[axis].min(pair[1] - pair[0]);
            }
        }
        let precision = feature;
        feature = feature.max(DVec2::splat(feature_floor));
        let span = max - min;
        let ratio = span / feature;
        if ratio.max_element() > 4096.0 {
            let axis = if ratio.x >= ratio.y { 0 } else { 1 };
            let mut coordinates = polygons
                .iter()
                .flatten()
                .map(|p| p[axis])
                .collect::<Vec<_>>();
            coordinates.sort_by(f64::total_cmp);
            coordinates.dedup();
            if let Some(pair) = coordinates
                .windows(2)
                .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
            {
                let cut = pair[0] + (pair[1] - pair[0]) * 0.5;
                if cut > min[axis] && cut < max[axis] {
                    let lower = polygons.iter().map(|p| clip(p, axis, cut, true)).collect();
                    let upper = polygons.iter().map(|p| clip(p, axis, cut, false)).collect();
                    pending.push(upper);
                    pending.push(lower);
                    continue;
                }
            }
        }
        let mut origin = (min + max) * 0.5;
        let mut scale = (span * 0.5).map(|v| if v == 0.0 { 1.0 } else { v });
        let mut rebased = false;
        for axis in 0..2 {
            let mut normalized = BTreeMap::<u32, f32>::new();
            let loses_coordinate = polygons.iter().flatten().any(|p| {
                let value = p[axis] as f32;
                let unit = ((f64::from(value) - origin[axis]) / scale[axis]) as f32;
                let key = if unit == 0.0 { 0 } else { unit.to_bits() };
                normalized
                    .insert(key, value)
                    .is_some_and(|old| old != value)
            });
            if feature_floor > 0.0 && loses_coordinate {
                rebased = true;
                // Binary scaling without translation preserves representable gaps near zero even for very wide strokes.
                let extent = min[axis].abs().max(max[axis].abs());
                origin[axis] = 0.0;
                scale[axis] = if extent == 0.0 {
                    1.0
                } else {
                    extent.log2().ceil().exp2()
                };
            }
        }
        // Lyon also uses tolerance for edge connections, so normalization must preserve the same feature budget.
        let mut tolerance = if rebased {
            ((precision / scale).min_element() * 0.125)
                .min(1.0 / 4096.0)
                .max(f64::from(f32::from_bits(1))) as f32
        } else {
            1.0 / 4096.0
        };
        if feature_floor > 0.0 {
            let face_spacing = center_edges
                .iter()
                .map(|edge| (*edge / scale).length())
                .fold(f64::INFINITY, f64::min);
            tolerance =
                tolerance.min((face_spacing * 0.125).max(f64::from(f32::from_bits(1))) as f32);
            tolerance = connection_tolerance(&polygons, origin, scale, tolerance);
        }
        let mut builder = lyon::path::Path::builder();
        let point = |p: DVec2| {
            let p = (p - origin) / scale;
            point(p.x as f32, p.y as f32)
        };
        for polygon in polygons {
            builder.begin(point(polygon[0]));
            for p in polygon.iter().skip(1) {
                builder.line_to(point(*p));
            }
            builder.end(true);
        }
        append_mesh(
            target,
            tessellate(&builder.build(), tolerance, rule)?,
            origin,
            scale,
        )?;
    }
    Ok(())
}

fn connection_tolerance(polygons: &[Vec<DVec2>], origin: DVec2, scale: DVec2, initial: f32) -> f32 {
    let mut polygons = polygons
        .iter()
        .map(|p| p.iter().map(|p| (*p - origin) / scale).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    // Degenerate clipped pieces cannot cover a connection interval in the painted region.
    polygons.retain(|p| {
        p[1..]
            .windows(2)
            .any(|v| (v[0] - p[0]).perp_dot(v[1] - p[0]) > 0.0)
    });
    let key = |p: DVec2| {
        (
            if p.x == 0.0 { 0 } else { p.x.to_bits() },
            if p.y == 0.0 { 0 } else { p.y.to_bits() },
        )
    };
    let mut owners = BTreeMap::<_, Vec<usize>>::new();
    for (i, polygon) in polygons.iter().enumerate() {
        for p in polygon {
            owners.entry(key(*p)).or_default().push(i);
        }
    }
    let bounds = polygons
        .iter()
        .map(|p| {
            (
                p.iter()
                    .copied()
                    .fold(DVec2::splat(f64::INFINITY), DVec2::min),
                p.iter()
                    .copied()
                    .fold(DVec2::splat(f64::NEG_INFINITY), DVec2::max),
            )
        })
        .collect::<Vec<_>>();
    let mut ordered = (0..bounds.len()).collect::<Vec<_>>();
    ordered.sort_by(|a, b| bounds[*a].0.x.total_cmp(&bounds[*b].0.x));
    let mut right = f64::NEG_INFINITY;
    let prefix_right = ordered
        .iter()
        .map(|i| {
            right = right.max(bounds[*i].1.x);
            right
        })
        .collect::<Vec<_>>();
    let mut tolerance = f64::from(initial);
    for ((x, y), incident) in owners {
        let p = DVec2::new(f64::from_bits(x), f64::from_bits(y));
        let end = ordered.partition_point(|i| bounds[*i].0.x <= p.x + tolerance);
        for slot in (0..end).rev() {
            if prefix_right[slot] < p.x - tolerance {
                break;
            }
            let j = ordered[slot];
            let other = &polygons[j];
            if !(p + DVec2::splat(tolerance)).cmpge(bounds[j].0).all()
                || !(p - DVec2::splat(tolerance)).cmple(bounds[j].1).all()
                || contains_point(other, p)
            {
                continue;
            }
            for (a, b) in other.iter().zip(other.iter().cycle().skip(1)) {
                let edge = *b - *a;
                let length = edge.length_squared();
                if length == 0.0 {
                    continue;
                }
                let along = ((p - *a).dot(edge) / length).clamp(0.0, 1.0);
                let nearest = if along == 0.0 {
                    *a
                } else if along == 1.0 {
                    *b
                } else {
                    *a + edge * along
                };
                let distance = p.distance(nearest);
                if distance > 0.0 && distance < tolerance {
                    // A connection already coincident in tessellator or final mesh coordinates cannot be separated by tolerance.
                    if p.as_vec2() == nearest.as_vec2()
                        || (origin + p * scale).as_vec2() == (origin + nearest * scale).as_vec2()
                    {
                        continue;
                    }
                    // Sharing a corner elsewhere does not make this gap an intentional join.
                    let covered = incident
                        .iter()
                        .any(|i| contains_point(&polygons[*i], nearest))
                        || covered_connection(p, nearest, &polygons, &bounds);
                    if !covered {
                        tolerance = distance * 0.125;
                    }
                }
            }
        }
    }
    tolerance.max(f64::from(f32::from_bits(1))) as f32
}

fn contains_point(polygon: &[DVec2], p: DVec2) -> bool {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .all(|(a, b)| (*b - *a).perp_dot(p - *a) >= 0.0)
}

fn covered_connection(
    from: DVec2,
    to: DVec2,
    polygons: &[Vec<DVec2>],
    bounds: &[(DVec2, DVec2)],
) -> bool {
    let min = from.min(to);
    let max = from.max(to);
    let mut intervals = Vec::new();
    for (polygon, (lo, hi)) in polygons.iter().zip(bounds) {
        if !max.cmpge(*lo).all() || !min.cmple(*hi).all() {
            continue;
        }
        let mut begin = 0.0_f64;
        let mut end = 1.0_f64;
        for (a, b) in polygon.iter().zip(polygon.iter().cycle().skip(1)) {
            let start = (*b - *a).perp_dot(from - *a);
            let finish = (*b - *a).perp_dot(to - *a);
            if start < 0.0 && finish < 0.0 {
                end = -1.0;
                break;
            }
            if start < 0.0 {
                begin = begin.max(start / (start - finish));
            } else if finish < 0.0 {
                end = end.min(start / (start - finish));
            }
        }
        if begin <= end {
            intervals.push((begin, end));
        }
    }
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut reached = 0.0_f64;
    for (begin, end) in intervals {
        if begin > reached {
            return false;
        }
        reached = reached.max(end);
        if reached >= 1.0 {
            return true;
        }
    }
    false
}

fn convex(polygon: &[DVec2]) -> bool {
    let mut sign = 0.0_f64;
    let mut turn = 0.0_f64;
    for i in 0..polygon.len() {
        let a = polygon[(i + 1) % polygon.len()] - polygon[i];
        let b = polygon[(i + 2) % polygon.len()] - polygon[(i + 1) % polygon.len()];
        let cross = a.perp_dot(b);
        if cross != 0.0 {
            if sign != 0.0 && sign != cross.signum() {
                return false;
            }
            sign = cross.signum();
        }
        turn += cross.atan2(a.dot(b));
    }
    sign != 0.0 && (turn.abs() - std::f64::consts::TAU).abs() < 1e-6
}

fn clip(polygon: &[DVec2], axis: usize, cut: f64, lower: bool) -> Vec<DVec2> {
    let inside = |p: DVec2| {
        if lower {
            p[axis] <= cut
        } else {
            p[axis] >= cut
        }
    };
    let mut output = Vec::new();
    if let Some(mut previous) = polygon.last().copied() {
        for current in polygon.iter().copied() {
            if inside(previous) != inside(current) {
                let (reference, other) =
                    if (cut - previous[axis]).abs() <= (cut - current[axis]).abs() {
                        (previous, current)
                    } else {
                        (current, previous)
                    };
                let t = (cut - reference[axis]) / (other[axis] - reference[axis]);
                let mut p = reference + (other - reference) * t;
                p[axis] = cut;
                output.push(p);
            }
            if inside(current) {
                output.push(current);
            }
            previous = current;
        }
    }
    output
}

fn append_mesh(
    target: &mut VertexBuffers<[f32; 3], u32>,
    source: VertexBuffers<[f32; 2], u32>,
    origin: DVec2,
    scale: DVec2,
) -> Result {
    let base = u32::try_from(target.vertices.len()).map_err(BevyError::error)?;
    for p in source.vertices {
        let p = (origin + DVec2::new(f64::from(p[0]), f64::from(p[1])) * scale).as_vec2();
        if !p.is_finite() {
            return Err(BevyError::error("Canvas Path mesh overflow"));
        }
        target.vertices.push([p.x, p.y, 0.0]);
    }
    for index in source.indices {
        target.indices.push(
            base.checked_add(index)
                .ok_or_else(|| BevyError::error("Canvas Path mesh indices overflow"))?,
        );
    }
    Ok(())
}

fn tessellate(
    path: &lyon::path::Path,
    tolerance: f32,
    rule: CanvasFillRule,
) -> Result<VertexBuffers<[f32; 2], u32>> {
    let mut vertices = VertexBuffers::new();
    FillTessellator::new()
        .tessellate_path(
            path,
            &FillOptions::default()
                .with_fill_rule(match rule {
                    CanvasFillRule::NonZero => FillRule::NonZero,
                    CanvasFillRule::EvenOdd => FillRule::EvenOdd,
                })
                .with_tolerance(tolerance),
            &mut BuffersBuilder::new(&mut vertices, |v: FillVertex| v.position().to_array()),
        )
        .map_err(BevyError::error)?;
    Ok(vertices)
}

fn append_stroke(
    polygons: &mut Vec<Vec<DVec2>>,
    center_edges: &mut Vec<DVec2>,
    points: &[DVec2],
    closed: bool,
    half: f64,
    cap: CanvasLineCap,
    join: CanvasLineJoin,
) {
    let mut points = points.to_vec();
    points.dedup();
    if closed && points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    let normal = |a: DVec2, b: DVec2| (b - a).normalize().perp() * half;
    let mut polygon = |mut points: Vec<DVec2>| {
        let a = points[0];
        let area = points[1..]
            .windows(2)
            .map(|p| (p[0] - a).perp_dot(p[1] - a))
            .sum::<f64>();
        if area == 0.0 {
            return;
        }
        if area < 0.0 {
            points.reverse();
        }
        polygons.push(points);
    };
    if points.len() < 2 {
        if let Some(p) = points.first().copied() {
            match cap {
                CanvasLineCap::Butt => {}
                CanvasLineCap::Round => polygon(sector(p, DVec2::X * half, std::f64::consts::TAU)),
                CanvasLineCap::Square => polygon(vec![
                    p + DVec2::splat(-half),
                    p + DVec2::new(half, -half),
                    p + DVec2::splat(half),
                    p + DVec2::new(-half, half),
                ]),
            }
        }
        return;
    }
    let count = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    for i in 0..count {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let n = normal(a, b);
        center_edges.push(b - a);
        // Centerline anchors survive even when the distant offset endpoints lose the short segment length.
        polygon(vec![a, b, b + n, a + n]);
        polygon(vec![b, a, a - n, b - n]);
        if !closed && i == 0 {
            continue;
        }
        let previous = points[(i + points.len() - 1) % points.len()];
        let before = normal(previous, a);
        let sum = before + n;
        let denominator = sum.dot(n);
        for sign in [-1.0, 1.0] {
            let p = a + before * sign;
            let q = a + n * sign;
            let miter = if denominator > 0.0 {
                sum * (half * half / denominator)
            } else {
                DVec2::ZERO
            };
            let outer = (a - previous).perp_dot(b - a) * sign < 0.0;
            if outer {
                match join {
                    CanvasLineJoin::Miter if miter.length() <= half * 4.0 && denominator > 0.0 => {
                        polygon(vec![a, p, a + miter * sign, q])
                    }
                    CanvasLineJoin::Round => {
                        let u = before * sign;
                        let v = n * sign;
                        polygon(sector(a, u, u.perp_dot(v).atan2(u.dot(v))));
                    }
                    CanvasLineJoin::Miter | CanvasLineJoin::Bevel => polygon(vec![a, p, q]),
                }
            }
        }
        if (a - previous).perp_dot(b - a) == 0.0
            && (a - previous).dot(b - a) < 0.0
            && join == CanvasLineJoin::Round
        {
            polygon(sector(a, n, std::f64::consts::PI));
        }
    }
    if !closed {
        for (p, toward) in [
            (points[0], points[1]),
            (points[points.len() - 1], points[points.len() - 2]),
        ] {
            let outward = (p - toward).normalize() * half;
            let side = outward.perp();
            match cap {
                CanvasLineCap::Butt => {}
                CanvasLineCap::Round => polygon(sector(p, -side, std::f64::consts::PI)),
                CanvasLineCap::Square => polygon(vec![
                    p - side,
                    p + outward - side,
                    p + outward + side,
                    p + side,
                ]),
            }
        }
    }
}

fn sector(center: DVec2, start: DVec2, angle: f64) -> Vec<DVec2> {
    let steps = (angle.abs() / std::f64::consts::TAU * 256.0)
        .ceil()
        .max(1.0) as usize;
    let mut points = Vec::with_capacity(steps + 2);
    points.push(center);
    for i in 0..=steps {
        let (sin, cos) = (angle * i as f64 / steps as f64).sin_cos();
        points.push(if i == steps && angle.abs() == std::f64::consts::TAU {
            center + start
        } else {
            center + start * cos + start.perp() * sin
        });
    }
    points
}
