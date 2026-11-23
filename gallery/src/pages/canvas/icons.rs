use bevy::prelude::*;
use bevy_widgetry::{canvas::*, job::JobFailure};

const SIGNAL_X: f32 = -220.0;
const LABEL_Y: f32 = 86.0;
const LAUNCH_X: f32 = 220.0;
const INK: Color = Color::srgb(0.91, 0.97, 1.0);
const LABEL: Color = Color::srgb(0.73, 0.81, 0.92);

type Result<T> = std::result::Result<T, JobFailure>;

fn stops(start: Color, end: Color) -> Result<CanvasGradientStops> {
    CanvasGradientStops::new(&[
        CanvasColorStop {
            offset: 0.0,
            color: start,
        },
        CanvasColorStop {
            offset: 1.0,
            color: end,
        },
    ])
}

fn linear(start: Color, end: Color) -> Result<CanvasFill> {
    Ok(CanvasFill::LinearGradient {
        start: Vec2::new(-56.0, -52.0),
        end: Vec2::new(56.0, 52.0),
        stops: stops(start, end)?,
    })
}

fn filled(fill: impl Into<CanvasFill>) -> CanvasPaint {
    CanvasPaint {
        fill: Some(fill.into()),
        ..default()
    }
}

fn vertices(points: usize, outer: f32, inner: f32) -> CanvasPath {
    let mut commands = (0..points)
        .map(|index| {
            let angle =
                index as f32 / points as f32 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let point = Vec2::from_angle(angle) * if index % 2 == 0 { outer } else { inner };
            if index == 0 {
                CanvasPathCommand::MoveTo(point)
            } else {
                CanvasPathCommand::LineTo(point)
            }
        })
        .collect::<Vec<_>>();
    commands.push(CanvasPathCommand::Close);
    CanvasPath { commands }
}

fn caption(
    world: &mut World,
    edit: CanvasEditToken,
    layer: CanvasLayerId,
    label: &str,
    x: f32,
) -> Result<Entity> {
    CanvasDocument::create_text_in_layer(
        world,
        edit,
        layer,
        CanvasTextStyle {
            content: label.into(),
            color: LABEL,
            font_size: 18.0,
            align: CanvasTextAlign::Center,
        },
        Transform::from_xyz(x, LABEL_Y, 0.0),
    )
}

pub(super) fn populate(world: &mut World, document: Entity, edit: CanvasEditToken) -> Result<()> {
    let accents = CanvasDocument::default_layer(world, document)?;
    CanvasDocument::rename_layer(world, edit, accents, "Accents")?;
    let icons = CanvasDocument::create_layer(world, edit, 1, "Icons")?;
    for (x, point_count, radii, color) in [
        (-335.0, 6, (10.0, 10.0), Color::srgb(0.32, 0.84, 0.79)),
        (335.0, 10, (16.0, 6.0), Color::srgb(1.0, 0.68, 0.36)),
    ] {
        CanvasDocument::create_path_in_layer(
            world,
            edit,
            accents,
            vertices(point_count, radii.0, radii.1),
            Transform::from_xyz(x, -46.0, 0.0),
            filled(color),
        )?;
    }
    signal(world, edit, icons)?;
    prism(world, edit, icons)?;
    launch(world, edit, icons)?;
    Ok(())
}

fn signal(world: &mut World, edit: CanvasEditToken, layer: CanvasLayerId) -> Result<()> {
    let pose = Transform::from_xyz(SIGNAL_X, 0.0, 0.0);
    let base = CanvasDocument::create_rectangle_in_layer(
        world,
        edit,
        layer,
        CanvasRectangle {
            size: Vec2::splat(114.0),
            corner_radius: 25.0,
        },
        pose,
        filled(linear(
            Color::srgb(0.11, 0.64, 0.65),
            Color::srgb(0.06, 0.24, 0.43),
        )?),
    )?;
    let pulse = CanvasDocument::create_path_in_layer(
        world,
        edit,
        layer,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::new(-38.0, 0.0)),
                CanvasPathCommand::LineTo(Vec2::new(-21.0, 0.0)),
                CanvasPathCommand::LineTo(Vec2::new(-10.0, -20.0)),
                CanvasPathCommand::LineTo(Vec2::new(6.0, 22.0)),
                CanvasPathCommand::LineTo(Vec2::new(18.0, 0.0)),
                CanvasPathCommand::LineTo(Vec2::new(37.0, 0.0)),
            ],
        },
        pose,
        CanvasPaint {
            fill: None,
            stroke: Some(CanvasStroke {
                color: INK,
                width: 5.0,
                cap: CanvasLineCap::Round,
                join: CanvasLineJoin::Round,
            }),
            ..default()
        },
    )?;
    let name = caption(world, edit, layer, "Signal", SIGNAL_X)?;
    CanvasDocument::group_items(world, edit, &[base, pulse, name], "Signal")?;
    Ok(())
}

fn prism(world: &mut World, edit: CanvasEditToken, layer: CanvasLayerId) -> Result<()> {
    let face = CanvasDocument::create_ellipse_in_layer(
        world,
        edit,
        layer,
        CanvasEllipse {
            radii: Vec2::splat(61.0),
        },
        Transform::IDENTITY,
        filled(CanvasFill::RadialGradient {
            center: Vec2::new(-15.0, -20.0),
            radius: 79.0,
            stops: stops(Color::srgb(0.62, 0.56, 1.0), Color::srgb(0.2, 0.12, 0.54))?,
        }),
    )?;
    let star = CanvasDocument::create_path_in_layer(
        world,
        edit,
        layer,
        vertices(8, 42.0, 12.0),
        Transform::IDENTITY.with_rotation(Quat::from_rotation_z(0.27)),
        filled(linear(INK, Color::srgb(0.48, 0.83, 0.99))?),
    )?;
    let dot = CanvasDocument::create_ellipse_in_layer(
        world,
        edit,
        layer,
        CanvasEllipse {
            radii: Vec2::splat(5.0),
        },
        Transform::IDENTITY,
        filled(Color::srgb(0.37, 0.20, 0.76)),
    )?;
    let name = caption(world, edit, layer, "Prism", 0.0)?;
    CanvasDocument::group_items(world, edit, &[face, star, dot, name], "Prism")?;
    Ok(())
}

fn launch(world: &mut World, edit: CanvasEditToken, layer: CanvasLayerId) -> Result<()> {
    let pose = Transform::from_xyz(LAUNCH_X, 0.0, 0.0);
    let button = CanvasDocument::create_rectangle_in_layer(
        world,
        edit,
        layer,
        CanvasRectangle {
            size: Vec2::new(130.0, 83.0),
            corner_radius: 30.0,
        },
        pose,
        filled(linear(
            Color::srgb(1.0, 0.63, 0.37),
            Color::srgb(0.84, 0.24, 0.38),
        )?),
    )?;
    let mark = CanvasDocument::create_path_in_layer(
        world,
        edit,
        layer,
        CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::new(-16.0, -18.0)),
                CanvasPathCommand::QuadTo {
                    control: Vec2::new(-16.0, -24.0),
                    to: Vec2::new(-9.0, -20.0),
                },
                CanvasPathCommand::LineTo(Vec2::new(21.0, -4.0)),
                CanvasPathCommand::QuadTo {
                    control: Vec2::new(29.0, 0.0),
                    to: Vec2::new(21.0, 4.0),
                },
                CanvasPathCommand::LineTo(Vec2::new(-9.0, 20.0)),
                CanvasPathCommand::QuadTo {
                    control: Vec2::new(-16.0, 24.0),
                    to: Vec2::new(-16.0, 18.0),
                },
                CanvasPathCommand::Close,
            ],
        },
        pose,
        filled(INK),
    )?;
    let name = caption(world, edit, layer, "Launch", LAUNCH_X)?;
    CanvasDocument::group_items(world, edit, &[button, mark, name], "Launch")?;
    Ok(())
}
