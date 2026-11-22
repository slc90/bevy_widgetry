use crate::{CanvasGeometry, CanvasPath, CanvasPathCommand::*, CanvasRectangle};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum CanvasFlowchartPreset {
    Terminator,
    Decision,
    InputOutput,
    Database,
    Document,
    Arrow,
}

impl CanvasFlowchartPreset {
    pub fn geometry(self, size: Vec2) -> Option<CanvasGeometry> {
        if !size.is_finite() || size.min_element() < 1.0 {
            return None;
        }
        Some(self.outline(size))
    }

    pub(crate) fn outline(self, size: Vec2) -> CanvasGeometry {
        let half = size * 0.5;
        let (x, y) = (half.x, half.y);
        let point = Vec2::new;
        let commands = match self {
            Self::Terminator => {
                return CanvasGeometry::Rectangle(CanvasRectangle {
                    size,
                    corner_radius: half.min_element(),
                });
            }
            Self::Decision => vec![
                MoveTo(point(0.0, -y)),
                LineTo(point(x, 0.0)),
                LineTo(point(0.0, y)),
                LineTo(point(-x, 0.0)),
                Close,
            ],
            Self::InputOutput => {
                let inset = x * 0.5;
                vec![
                    MoveTo(point(-x + inset, -y)),
                    LineTo(point(x, -y)),
                    LineTo(point(x - inset, y)),
                    LineTo(point(-x, y)),
                    Close,
                ]
            }
            Self::Database => {
                let cap_height = y * 0.3;
                let top = -y + cap_height;
                let bottom = y - cap_height;
                let arc = |to, sweep| ArcTo {
                    radii: point(x, cap_height),
                    rotation: 0.0,
                    large_arc: false,
                    sweep,
                    to,
                };
                let left = point(-x, top);
                let right = point(x, top);
                // An open curve implicitly closes for filling and cuts a hole under EvenOdd.
                // A diameter marks the cap without contributing a second filled region.
                vec![
                    MoveTo(right),
                    LineTo(left),
                    MoveTo(left),
                    arc(right, true),
                    LineTo(point(x, bottom)),
                    arc(point(-x, bottom), true),
                    Close,
                ]
            }
            Self::Document => {
                let wave_height = y * 0.4;
                let trough = y - wave_height;
                let middle = y - wave_height * 0.5;
                vec![
                    MoveTo(point(-x, -y)),
                    LineTo(point(x, -y)),
                    LineTo(point(x, middle)),
                    CubicTo {
                        control1: point(x * 0.75, middle),
                        control2: point(x * 0.75, y),
                        to: point(x * 0.5, y),
                    },
                    CubicTo {
                        control1: point(0.0, y),
                        control2: point(0.0, trough),
                        to: point(-x * 0.5, trough),
                    },
                    CubicTo {
                        control1: point(-x * 0.75, trough),
                        control2: point(-x * 0.75, middle),
                        to: point(-x, middle),
                    },
                    Close,
                ]
            }
            Self::Arrow => {
                let shaft = y * 0.45;
                let neck = x * 0.2;
                vec![
                    MoveTo(point(-x, -shaft)),
                    LineTo(point(neck, -shaft)),
                    LineTo(point(neck, -y)),
                    LineTo(point(x, 0.0)),
                    LineTo(point(neck, y)),
                    LineTo(point(neck, shaft)),
                    LineTo(point(-x, shaft)),
                    Close,
                ]
            }
        };
        CanvasGeometry::Path(CanvasPath { commands })
    }
}
