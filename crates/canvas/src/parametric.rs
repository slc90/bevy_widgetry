use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Reflect)]
pub enum CanvasPathTool {
    RegularPolygon {
        sides: u8,
    },
    Star {
        points: u8,
        inner_ratio: f32,
    },
    Arc {
        radii: Vec2,
        rotation: f32,
        large_arc: bool,
        sweep: bool,
    },
}

impl CanvasPathTool {
    pub(crate) fn valid(self) -> bool {
        match self {
            Self::RegularPolygon { sides } => sides >= 3,
            Self::Star {
                points,
                inner_ratio,
            } => points >= 3 && inner_ratio > 0.0 && inner_ratio < 1.0,
            Self::Arc {
                radii, rotation, ..
            } => radii.is_finite() && radii.min_element() > 0.0 && rotation.is_finite(),
        }
    }

    pub(crate) fn radial(self) -> bool {
        matches!(self, Self::RegularPolygon { .. } | Self::Star { .. })
    }

    pub(crate) fn accepts_drag(self, delta: Vec2) -> bool {
        if delta == Vec2::ZERO || !delta.is_finite() {
            return false;
        }
        match self {
            Self::Arc {
                radii, rotation, ..
            } => {
                let half_chord = bevy::math::DVec2::from_angle(-f64::from(rotation))
                    .rotate(delta.as_dvec2())
                    * 0.5;
                // The endpoints and rotation arrive as f32, including rounded quarter turns.
                (half_chord / radii.as_dvec2()).length_squared() <= 1.0 + f64::from(f32::EPSILON)
            }
            _ => delta.length().is_finite(),
        }
    }

    pub(crate) fn path(self, delta: Vec2) -> crate::CanvasPath {
        use crate::CanvasPathCommand::*;
        let (count, inner) = match self {
            Self::RegularPolygon { sides } => (usize::from(sides), None),
            Self::Star {
                points,
                inner_ratio,
            } => (2 * usize::from(points), Some(inner_ratio)),
            Self::Arc {
                radii,
                rotation,
                large_arc,
                sweep,
            } => {
                return crate::CanvasPath {
                    commands: vec![
                        MoveTo(-delta * 0.5),
                        ArcTo {
                            radii,
                            rotation,
                            large_arc,
                            sweep,
                            to: delta * 0.5,
                        },
                    ],
                };
            }
        };
        let mut commands = Vec::with_capacity(count + 1);
        commands.push(MoveTo(delta));
        for i in 1..count {
            let angle = std::f32::consts::TAU * i as f32 / count as f32;
            let radius_scale = if i % 2 == 1 {
                inner.unwrap_or(1.0)
            } else {
                1.0
            };
            commands.push(LineTo(Vec2::from_angle(angle).rotate(delta) * radius_scale));
        }
        commands.push(Close);
        crate::CanvasPath { commands }
    }
}
