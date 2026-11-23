use bevy::prelude::*;
use bevy_widgetry::{canvas::*, job::JobFailure};

pub(super) type Result<T = ()> = std::result::Result<T, JobFailure>;

pub(super) const INK: Color = Color::srgb_u8(37, 34, 77);
pub(super) const LILAC: Color = Color::srgb_u8(160, 144, 221);
pub(super) const SILVER: Color = Color::srgb_u8(248, 237, 255);
pub(super) const CYAN: Color = Color::srgb_u8(111, 246, 237);
pub(super) const VIOLET: Color = Color::srgb_u8(116, 97, 214);
pub(super) const SKIN: Color = Color::srgb_u8(255, 210, 201);
pub(super) const LIGHT_SKIN: Color = Color::srgb_u8(255, 238, 224);

pub(super) const CLOSE: CanvasPathCommand = CanvasPathCommand::Close;

pub(super) fn m(x: f32, y: f32) -> CanvasPathCommand {
    CanvasPathCommand::MoveTo(Vec2::new(x, y))
}

pub(super) fn l(x: f32, y: f32) -> CanvasPathCommand {
    CanvasPathCommand::LineTo(Vec2::new(x, y))
}

pub(super) fn c(a: (f32, f32), b: (f32, f32), to: (f32, f32)) -> CanvasPathCommand {
    CanvasPathCommand::CubicTo {
        control1: Vec2::from(a),
        control2: Vec2::from(b),
        to: Vec2::from(to),
    }
}

pub(super) fn line(color: Color, width: f32) -> CanvasPaint {
    CanvasPaint {
        fill: None,
        stroke: Some(CanvasStroke {
            color,
            width,
            cap: CanvasLineCap::Round,
            join: CanvasLineJoin::Round,
        }),
        ..default()
    }
}

pub(super) fn fill(color: impl Into<CanvasFill>) -> CanvasPaint {
    CanvasPaint {
        fill: Some(color.into()),
        ..default()
    }
}

pub(super) fn outlined(color: impl Into<CanvasFill>, border: Color, width: f32) -> CanvasPaint {
    CanvasPaint {
        fill: Some(color.into()),
        ..line(border, width)
    }
}

pub(super) fn gradient(from: ((f32, f32), Color), to: ((f32, f32), Color)) -> Result<CanvasFill> {
    Ok(CanvasFill::LinearGradient {
        start: Vec2::from(from.0),
        end: Vec2::from(to.0),
        stops: CanvasGradientStops::new(&[
            CanvasColorStop {
                offset: 0.0,
                color: from.1,
            },
            CanvasColorStop {
                offset: 1.0,
                color: to.1,
            },
        ])?,
    })
}

pub(super) struct Drawing<'w> {
    pub(super) world: &'w mut World,
    pub(super) edit: CanvasEditToken,
    pub(super) layer: CanvasLayerId,
    pub(super) origin: Vec2,
}

impl<'w> Drawing<'w> {
    pub(super) fn new(world: &'w mut World, edit: CanvasEditToken, layer: CanvasLayerId) -> Self {
        Self {
            world,
            edit,
            layer,
            origin: Vec2::ZERO,
        }
    }

    pub(super) fn path(
        &mut self,
        paint: CanvasPaint,
        commands: impl Into<Vec<CanvasPathCommand>>,
    ) -> Result<Entity> {
        CanvasDocument::create_path_in_layer(
            self.world,
            self.edit,
            self.layer,
            CanvasPath {
                commands: commands.into(),
            },
            Transform::from_translation(self.origin.extend(0.0)),
            paint,
        )
    }

    pub(super) fn rect(
        &mut self,
        center: (f32, f32),
        size: (f32, f32),
        radius: f32,
        paint: CanvasPaint,
    ) -> Result<Entity> {
        CanvasDocument::create_rectangle_in_layer(
            self.world,
            self.edit,
            self.layer,
            CanvasRectangle {
                size: Vec2::from(size),
                corner_radius: radius,
            },
            pose(self.origin, center),
            paint,
        )
    }

    pub(super) fn ellipse(
        &mut self,
        center: (f32, f32),
        radii: (f32, f32),
        paint: CanvasPaint,
    ) -> Result<Entity> {
        CanvasDocument::create_ellipse_in_layer(
            self.world,
            self.edit,
            self.layer,
            CanvasEllipse {
                radii: Vec2::from(radii),
            },
            pose(self.origin, center),
            paint,
        )
    }

    pub(super) fn text(
        &mut self,
        at: (f32, f32),
        content: &str,
        size: f32,
        color: Color,
    ) -> Result<Entity> {
        CanvasDocument::create_text_in_layer(
            self.world,
            self.edit,
            self.layer,
            CanvasTextStyle {
                content: content.into(),
                color,
                font_size: size,
                align: CanvasTextAlign::Center,
            },
            pose(self.origin, at),
        )
    }

    pub(super) fn group(&mut self, name: &str, parts: &[Entity]) -> Result<Entity> {
        CanvasDocument::group_items(self.world, self.edit, parts, name)
    }
}

fn pose(origin: Vec2, (x, y): (f32, f32)) -> Transform {
    Transform::from_translation((origin + Vec2::new(x, y)).extend(0.0))
}
