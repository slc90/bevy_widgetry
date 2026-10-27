use bevy::{
    math::{DMat2, DVec2},
    mesh::VertexAttributeValues,
    prelude::*,
    ui::{ComputedUiTargetCamera, UiGlobalTransform},
};

const STROKE_TOLERANCE: f64 = 4.0;

pub(super) struct ScreenSpace {
    linear: DMat2,
}

impl ScreenSpace {
    pub(super) fn new(world: &World, view: Entity, zoom: f32) -> Result<Self> {
        let node = world
            .get::<ComputedNode>(view)
            .ok_or_else(|| BevyError::error("Canvas hit layout missing"))?;
        let ui = world
            .get::<UiGlobalTransform>(view)
            .ok_or_else(|| BevyError::error("Canvas hit UI transform missing"))?;
        let camera = world
            .get::<ComputedUiTargetCamera>(view)
            .and_then(ComputedUiTargetCamera::get)
            .and_then(|camera| world.get::<Camera>(camera))
            .ok_or_else(|| BevyError::error("Canvas hit UI camera missing"))?;
        let scale = camera
            .target_scaling_factor()
            .ok_or_else(|| BevyError::error("Canvas hit target scale missing"))?;
        Ok(Self {
            linear: DMat2::from_cols(
                ui.transform_vector2(Vec2::X).as_dvec2(),
                ui.transform_vector2(Vec2::Y).as_dvec2(),
            ) * (f64::from(zoom)
                / (f64::from(node.inverse_scale_factor()) * f64::from(scale))),
        })
    }

    pub(super) fn near_bounds(&self, bounds: Rect, transform: &Transform, point: Vec2) -> bool {
        let x = (transform.rotation * Vec3::X * transform.scale.x)
            .truncate()
            .as_dvec2();
        let y = (transform.rotation * Vec3::Y * transform.scale.y)
            .truncate()
            .as_dvec2();
        let offset = transform.translation.truncate().as_dvec2() - point.as_dvec2();
        let points = [
            bounds.min,
            Vec2::new(bounds.max.x, bounds.min.y),
            bounds.max,
            Vec2::new(bounds.min.x, bounds.max.y),
        ]
        .map(|p| self.linear * (offset + x * f64::from(p.x) + y * f64::from(p.y)));
        let min = points
            .into_iter()
            .fold(DVec2::splat(f64::INFINITY), DVec2::min);
        let max = points
            .into_iter()
            .fold(DVec2::splat(f64::NEG_INFINITY), DVec2::max);
        min.cmple(DVec2::splat(STROKE_TOLERANCE)).all()
            && max.cmpge(DVec2::splat(-STROKE_TOLERANCE)).all()
    }

    pub(super) fn mesh(
        &self,
        mesh: &Mesh,
        transform: &Transform,
        point: Vec2,
        stroke: bool,
    ) -> Result<bool> {
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            return Err(BevyError::error("Canvas hit mesh positions missing"));
        };
        let indices = mesh
            .indices()
            .ok_or_else(|| BevyError::error("Canvas hit mesh indices missing"))?;
        let mut indices = indices.iter();
        let model = DMat2::from_cols(
            (transform.rotation * Vec3::X * transform.scale.x)
                .truncate()
                .as_dvec2(),
            (transform.rotation * Vec3::Y * transform.scale.y)
                .truncate()
                .as_dvec2(),
        );
        let linear = self.linear * model;
        // Subtract translations before projecting so a distant contour does not erase local precision.
        let offset = self.linear * (transform.translation.truncate().as_dvec2() - point.as_dvec2());
        let project = |index: usize| -> Result<DVec2> {
            let p = positions
                .get(index)
                .ok_or_else(|| BevyError::error("Canvas hit mesh index out of bounds"))?;
            Ok(linear * DVec2::new(f64::from(p[0]), f64::from(p[1])) + offset)
        };
        while let Some(a) = indices.next() {
            let b = indices
                .next()
                .ok_or_else(|| BevyError::error("Canvas hit triangle incomplete"))?;
            let c = indices
                .next()
                .ok_or_else(|| BevyError::error("Canvas hit triangle incomplete"))?;
            let [a, b, c] = [project(a)?, project(b)?, project(c)?];
            if (b - a).perp_dot(c - a) == 0.0 {
                continue;
            }
            let crosses = [
                (b - a).perp_dot(-a),
                (c - b).perp_dot(-b),
                (a - c).perp_dot(-c),
            ];
            if crosses.iter().all(|v| *v >= 0.0) || crosses.iter().all(|v| *v <= 0.0) {
                return Ok(true);
            }
            if stroke
                && [(a, b), (b, c), (c, a)].into_iter().any(|(a, b)| {
                    let edge = b - a;
                    let t = if edge.length_squared() > 0.0 {
                        (-a).dot(edge) / edge.length_squared()
                    } else {
                        0.0
                    };
                    (a + edge * t.clamp(0.0, 1.0)).length_squared()
                        <= STROKE_TOLERANCE * STROKE_TOLERANCE
                })
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
