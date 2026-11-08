const VIEW_HEIGHT: f32 = 340.0;

const ITEM_GAP: f32 = 4.0;
const GROUP_GAP: f32 = 8.0;
const SECTION_GAP: f32 = 12.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 12.0;

use crate::{
    gallery::{GalleryPage, mount_page, unmount_page},
    i18n::{CanvasKey, LocalizedText, TextKey, localized, localized_args},
};
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{button::WidgetryButton, canvas::*, text::WidgetryText};
use std::path::PathBuf;

pub(crate) struct CanvasDemoPlugin;

impl Plugin for CanvasDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CanvasPlugin)
            .add_systems(OnEnter(GalleryPage::Canvas), enter)
            .add_systems(OnExit(GalleryPage::Canvas), exit)
            .add_systems(Last, continue_exports)
            .add_systems(Update, refresh_geometry)
            .add_observer(png_finished)
            .add_observer(|_: On<CanvasPenChanged>, mut commands: Commands| commands.queue(refresh))
            .add_observer(|_: On<CanvasZoomChanged>, mut commands: Commands| {
                commands.queue(refresh)
            })
            .add_observer(|_: On<CanvasCenterChanged>, mut commands: Commands| {
                commands.queue(refresh)
            })
            .add_observer(|_: On<CanvasSelectionChanged>, mut commands: Commands| {
                commands.queue(refresh)
            })
            .add_observer(|_: On<CanvasModeChanged>, mut commands: Commands| {
                commands.queue(refresh)
            });
    }
}

#[derive(Component, FromTemplate, Clone)]
struct Demo {
    document: Entity,
    main: Entity,
    auxiliary: Entity,
    readout: Entity,
    result: Entity,
    directory: PathBuf,
    region: Option<Rect>,
    scale: f32,
    segment: usize,
}

#[derive(Component)]
struct PendingExport {
    result: Entity,
}

fn continue_exports(
    pending: Query<(), With<PendingExport>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    if !pending.is_empty() {
        redraw.write(bevy::window::RequestRedraw);
    }
}

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Select,
    Rectangle,
    Ellipse,
    Pen,
    Nodes,
    Add,
    Raise,
    Lower,
    Front,
    Back,
    Append,
    Previous,
    Next,
    Insert,
    Remove,
    Line,
    Quadratic,
    Cubic,
    Close,
    ZoomIn,
    ZoomOut,
    Reset,
    ToggleAuxiliary,
    Region,
    Scale,
    Svg,
    Png,
}

fn paint() -> CanvasPaint {
    CanvasPaint {
        fill: Some(Color::srgba(0.1, 0.75, 0.85, 0.55)),
        stroke: Some(CanvasStroke {
            color: Color::srgba(0.3, 0.5, 1.0, 0.8),
            width: 10.0,
            join: CanvasLineJoin::Bevel,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn refresh_geometry(
    shapes: Query<
        (),
        (
            With<CanvasShape>,
            Or<(Changed<CanvasGeometry>, Changed<Transform>)>,
        ),
    >,
    demos: Query<(), With<Demo>>,
    mut commands: Commands,
) {
    if !demos.is_empty() && !shapes.is_empty() {
        commands.queue(refresh);
    }
}

fn add_rectangle(world: &mut World, document: Entity) -> Result {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            corner_radius: 12.0,
            size: Vec2::new(120.0, 80.0),
        },
        Transform::default(),
        paint(),
    )
    .map_err(|error| error.into_bevy_error())?;
    Ok(())
}

fn enter(world: &mut World) -> Result {
    let directory = std::env::current_exe()
        .map_err(|error| {
            crate::diagnostics::at(
                error,
                bevy_widgetry::diagnostics::ErrorFrame::new("无法确定 Canvas 导出目录"),
            )
        })?
        .with_file_name("exports");
    let document = world
        .spawn((Name::new("CanvasDemoDocument"), CanvasDocument))
        .id();
    let result = (|| -> Result<Entity> {
        add_rectangle(world, document)?;
        CanvasDocument::create_ellipse(
            world,
            document,
            CanvasEllipse {
                radii: Vec2::new(40.0, 25.0),
            },
            Transform::from_xyz(150.0, 0.0, 0.0),
            CanvasPaint {
                fill: Some(Color::srgba(1.0, 0.5, 0.1, 0.7)),
                stroke: Some(CanvasStroke {
                    color: Color::srgb(1.0, 0.8, 0.2),
                    width: 4.0,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::create_ellipse(
            world,
            document,
            CanvasEllipse {
                radii: Vec2::new(7.0, 5.0),
            },
            Transform::from_xyz(-153.0, 0.0, 0.0),
            CanvasPaint {
                fill: Some(Color::srgb(1.0, 0.5, 0.1)),
                ..default()
            },
        )
        .map_err(|error| error.into_bevy_error())?;
        let path = CanvasDocument::create_path(
            world,
            document,
            CanvasPath {
                commands: vec![
                    CanvasPathCommand::MoveTo(Vec2::new(-45.0, 25.0)),
                    CanvasPathCommand::QuadTo {
                        control: Vec2::new(-45.0, -35.0),
                        to: Vec2::new(0.0, -35.0),
                    },
                    CanvasPathCommand::CubicTo {
                        control1: Vec2::new(45.0, -35.0),
                        control2: Vec2::new(45.0, 25.0),
                        to: Vec2::new(0.0, 25.0),
                    },
                    CanvasPathCommand::Close,
                    CanvasPathCommand::MoveTo(Vec2::new(-20.0, 45.0)),
                    CanvasPathCommand::ArcTo {
                        radii: Vec2::new(20.0, 12.0),
                        rotation: 0.0,
                        large_arc: false,
                        sweep: true,
                        to: Vec2::new(20.0, 45.0),
                    },
                ],
            },
            Transform::from_xyz(-150.0, 0.0, 0.0),
            CanvasPaint {
                fill: Some(Color::srgba(0.65, 0.3, 1.0, 0.6)),
                fill_rule: CanvasFillRule::EvenOdd,
                stroke: Some(CanvasStroke {
                    color: Color::srgb(0.85, 0.6, 1.0),
                    width: 6.0,
                    cap: CanvasLineCap::Round,
                    join: CanvasLineJoin::Round,
                }),
            },
        )
        .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_rectangle(
            world,
            document,
            path,
            Rect::new(-16.0, -10.0, 10.0, 10.0),
        )
        .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_circle(world, document, path, Vec2::new(-55.0, 75.0), 8.0)
            .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_ellipse(
            world,
            document,
            path,
            Vec2::new(-25.0, 75.0),
            Vec2::new(12.0, 6.0),
        )
        .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_line(
            world,
            document,
            path,
            Vec2::new(0.0, 75.0),
            Vec2::new(15.0, 75.0),
        )
        .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_polyline(
            world,
            document,
            path,
            &[
                Vec2::new(25.0, 80.0),
                Vec2::new(33.0, 65.0),
                Vec2::new(40.0, 80.0),
            ],
        )
        .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_polygon(
            world,
            document,
            path,
            &[
                Vec2::new(50.0, 80.0),
                Vec2::new(58.0, 65.0),
                Vec2::new(66.0, 80.0),
            ],
        )
        .map_err(|error| error.into_bevy_error())?;
        mount_page(
            world,
            GalleryPage::Canvas,
            bsn_list! { @scene(document, directory) },
        )
    })();
    match result {
        Ok(page) => {
            world.entity_mut(document).insert(ChildOf(page));
            refresh(world)
        }
        Err(error) => {
            world.despawn(document);
            Err(error)
        }
    }
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Canvas)
}

fn scene(document: Entity, directory: PathBuf) -> impl Scene {
    let path = directory.display().to_string();
    let controls: Vec<_> = [
        (CanvasKey::Select, Action::Select), (CanvasKey::Rectangle, Action::Rectangle),
        (CanvasKey::Ellipse, Action::Ellipse),
        (CanvasKey::Pen, Action::Pen),
        (CanvasKey::Nodes, Action::Nodes),
        (CanvasKey::Raise, Action::Raise), (CanvasKey::Lower, Action::Lower),
        (CanvasKey::Front, Action::Front), (CanvasKey::Back, Action::Back),
        (CanvasKey::Append, Action::Append),
        (CanvasKey::Previous, Action::Previous), (CanvasKey::Next, Action::Next),
        (CanvasKey::Insert, Action::Insert), (CanvasKey::Remove, Action::Remove),
        (CanvasKey::Line, Action::Line), (CanvasKey::Quadratic, Action::Quadratic),
        (CanvasKey::Cubic, Action::Cubic), (CanvasKey::Close, Action::Close),
        (CanvasKey::Add, Action::Add), (CanvasKey::ZoomIn, Action::ZoomIn),
        (CanvasKey::ZoomOut, Action::ZoomOut), (CanvasKey::Reset, Action::Reset),
        (CanvasKey::Auxiliary, Action::ToggleAuxiliary), (CanvasKey::Region, Action::Region),
        (CanvasKey::Scale, Action::Scale), (CanvasKey::Svg, Action::Svg), (CanvasKey::Png, Action::Png),
    ].into_iter().map(|(label, action)| bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Canvas(label)) WidgetryText}) as Box<dyn SceneList>)} }
        Name({format!("Canvas{action:?}")}) template(move |_| Ok(action)) on(operate)
    }).collect();
    bsn! {
        Name("CanvasDemo")
        Demo { document:document, main:#Main, auxiliary:#Auxiliary, readout:#Readout, result:#Result,
            directory:directory, region:None, scale:1.0, segment:0 }
        Node { width:percent(100), flex_direction:FlexDirection::Column, row_gap:px(SECTION_GAP), flex_shrink:0.0 }
        Children [
            @localized(TextKey::Canvas(CanvasKey::Description)) WidgetryText Node {max_width:percent(100)}--
            Node {column_gap:px(CONTROL_GAP),row_gap:px(GROUP_GAP),flex_wrap:FlexWrap::Wrap,max_width:percent(100)} Children [{controls}]--
            #Readout Name("CanvasReadout") Text("") WidgetryText Node {max_width:percent(100)}--
            @localized(TextKey::Canvas(CanvasKey::Editing)) WidgetryText Node {max_width:percent(100)}--
            Node {width:percent(100),column_gap:px(CONTENT_GAP)}
            Children [
                Node {flex_grow:2.0,flex_basis:px(0),min_width:px(0),row_gap:px(ITEM_GAP),flex_direction:FlexDirection::Column}
                Children [
                    @localized(TextKey::Canvas(CanvasKey::Main)) WidgetryText--
                    #Main Name("CanvasMain") @CanvasView {@document:{Some(document)}}
                    Node {width:percent(100),height:px(VIEW_HEIGHT),flex_shrink:0.0}
                    BackgroundColor(Color::srgb(0.09,0.11,0.16))
                ]--
                Node {flex_grow:1.0,flex_basis:px(0),min_width:px(0),row_gap:px(ITEM_GAP),flex_direction:FlexDirection::Column}
                Children [
                    @localized(TextKey::Canvas(CanvasKey::Secondary)) WidgetryText--
                    #Auxiliary Name("CanvasAuxiliary") @CanvasView {@document:{Some(document)},@zoom:0.5,@center:Vec2::new(20.0,-10.0)}
                    Node {width:percent(100),height:px(VIEW_HEIGHT),flex_shrink:0.0}
                    BackgroundColor(Color::srgb(0.13,0.1,0.16))
                ]
            ]--
            @localized_args(TextKey::Canvas(CanvasKey::Paths),vec![("path",path)]) WidgetryText Node {max_width:percent(100)}--
            #Result Name("CanvasExportResult") @localized(TextKey::Canvas(CanvasKey::Ready)) WidgetryText Node {max_width:percent(100)}
        ]
    }
}

fn invalid(reason: &str) -> BevyError {
    crate::diagnostics::at(
        reason.to_owned(),
        bevy_widgetry::diagnostics::ErrorFrame::new("Canvas 演示内部结构失效"),
    )
}

fn bind(world: &mut World, entity: Entity, binding: LocalizedText) -> Result {
    let mut label = world
        .get_entity_mut(entity)
        .map_err(|_| invalid("Canvas label missing"))?;
    label.insert(binding);
    Ok(())
}

struct Segment {
    command: usize,
    from: Vec2,
    to: Vec2,
}

fn selected_path(world: &World, demo: &Demo) -> Result<Option<(Entity, CanvasPath, Vec<Segment>)>> {
    let Some(shape) =
        CanvasView::single_selection(world, demo.main).map_err(|error| error.into_bevy_error())?
    else {
        return Ok(None);
    };
    let Some(CanvasGeometry::Path(path)) = world.get::<CanvasGeometry>(shape) else {
        return Ok(None);
    };
    let mut segments = Vec::new();
    let mut from = Vec2::ZERO;
    let mut start = from;
    for (command, value) in path.commands.iter().enumerate() {
        use CanvasPathCommand::*;
        let to = match *value {
            MoveTo(point) => {
                start = point;
                from = point;
                continue;
            }
            Close => {
                from = start;
                continue;
            }
            LineTo(to) | QuadTo { to, .. } | CubicTo { to, .. } | ArcTo { to, .. } => to,
        };
        segments.push(Segment { command, from, to });
        from = to;
    }
    Ok(Some((shape, path.clone(), segments)))
}

fn edit_path(world: &mut World, root: Entity, demo: &Demo, action: Action) -> Result {
    let Some((shape, path, segments)) = selected_path(world, demo)? else {
        return bind(
            world,
            demo.result,
            LocalizedText::new(TextKey::Canvas(CanvasKey::ChoosePath)),
        );
    };
    let index = demo.segment.min(segments.len().saturating_sub(1));
    if matches!(action, Action::Append) {
        let bounds = path
            .bounds(
                &Transform::default(),
                &CanvasPaint {
                    fill: None,
                    stroke: None,
                    ..default()
                },
            )
            .map_err(|error| error.into_bevy_error())?;
        CanvasDocument::append_circle(
            world,
            demo.document,
            shape,
            bounds.max + Vec2::splat(20.0),
            12.0,
        )
        .map_err(|error| error.into_bevy_error())?;
        return Ok(());
    }
    let Some(segment) = segments.get(index) else {
        return bind(
            world,
            demo.result,
            LocalizedText::new(TextKey::Canvas(CanvasKey::ChoosePath)),
        );
    };
    let command = segment.command;
    let middle = segment.from * 0.5 + segment.to * 0.5;
    let offset = (segment.to - segment.from).perp().normalize_or(Vec2::Y) * 30.0;
    match action {
        Action::Previous | Action::Next => {
            let next = if matches!(action, Action::Next) {
                (index + 1) % segments.len()
            } else {
                (index + segments.len() - 1) % segments.len()
            };
            world
                .get_mut::<Demo>(root)
                .ok_or_else(|| invalid("demo missing"))?
                .segment = next;
        }
        Action::Insert => {
            if !matches!(path.commands[command], CanvasPathCommand::LineTo(_)) {
                return bind(
                    world,
                    demo.result,
                    LocalizedText::new(TextKey::Canvas(CanvasKey::ChooseLine)),
                );
            }
            CanvasDocument::insert_path_anchor(world, demo.document, shape, command, middle)
                .map_err(|error| error.into_bevy_error())?;
        }
        Action::Remove => {
            CanvasDocument::remove_path_anchor(world, demo.document, shape, command)
                .map_err(|error| error.into_bevy_error())?;
        }
        Action::Line | Action::Quadratic | Action::Cubic => {
            let replacement = match action {
                Action::Quadratic => CanvasPathSegment::Quadratic {
                    control: middle + offset,
                },
                Action::Cubic => CanvasPathSegment::Cubic {
                    control1: segment.from.lerp(segment.to, 1.0 / 3.0) + offset,
                    control2: segment.from.lerp(segment.to, 2.0 / 3.0) + offset,
                },
                _ => CanvasPathSegment::Line,
            };
            CanvasDocument::set_path_segment(world, demo.document, shape, command, replacement)
                .map_err(|error| error.into_bevy_error())?;
        }
        Action::Close => {
            let closed = path.commands[command..]
                .iter()
                .find(|c| matches!(c, CanvasPathCommand::MoveTo(_) | CanvasPathCommand::Close))
                .is_some_and(|c| matches!(c, CanvasPathCommand::Close));
            CanvasDocument::set_path_subpath_closed(world, demo.document, shape, command, !closed)
                .map_err(|error| error.into_bevy_error())?;
        }
        _ => {}
    }
    Ok(())
}

fn refresh(world: &mut World) -> Result {
    let demos: Vec<_> = world.query::<&Demo>().iter(world).cloned().collect();
    for demo in demos {
        let a = CanvasView::center(world, demo.main).map_err(|error| error.into_bevy_error())?;
        let b =
            CanvasView::center(world, demo.auxiliary).map_err(|error| error.into_bevy_error())?;
        let segment = selected_path(world, &demo)?
            .and_then(|(_, path, segments)| {
                let index = demo.segment.min(segments.len().saturating_sub(1));
                segments.get(index).map(|segment| {
                    let kind = match path.commands[segment.command] {
                        CanvasPathCommand::LineTo(_) => "L",
                        CanvasPathCommand::QuadTo { .. } => "Q",
                        CanvasPathCommand::CubicTo { .. } => "C",
                        CanvasPathCommand::ArcTo { .. } => "A",
                        _ => "",
                    };
                    format!(
                        "{}/{} {} ({:.0},{:.0}) → ({:.0},{:.0})",
                        index + 1,
                        segments.len(),
                        kind,
                        segment.from.x,
                        segment.from.y,
                        segment.to.x,
                        segment.to.y
                    )
                })
            })
            .unwrap_or_else(|| "—".to_owned());
        let draft =
            CanvasView::pen_draft(world, demo.main).map_err(|error| error.into_bevy_error())?;
        let mode =
            match CanvasView::mode(world, demo.main).map_err(|error| error.into_bevy_error())? {
                CanvasMode::Select => CanvasKey::Select,
                CanvasMode::PathNodeEdit => CanvasKey::Nodes,
                CanvasMode::Rectangle(_) => CanvasKey::Rectangle,
                CanvasMode::Ellipse(_) => CanvasKey::Ellipse,
                CanvasMode::Pen(_) => CanvasKey::Pen,
            };
        let binding = LocalizedText::with_args(
            TextKey::Canvas(CanvasKey::Readout),
            vec![
                (
                    "zoom_a",
                    format!(
                        "{:.2}",
                        CanvasView::zoom(world, demo.main)
                            .map_err(|error| error.into_bevy_error())?
                    ),
                ),
                (
                    "zoom_b",
                    format!(
                        "{:.2}",
                        CanvasView::zoom(world, demo.auxiliary)
                            .map_err(|error| error.into_bevy_error())?
                    ),
                ),
                ("center_a", format!("{:.1}, {:.1}", a.x, a.y)),
                ("center_b", format!("{:.1}, {:.1}", b.x, b.y)),
                ("scale", demo.scale.to_string()),
                (
                    "selection_a",
                    CanvasView::selection(world, demo.main)
                        .map_err(|error| error.into_bevy_error())?
                        .len()
                        .to_string(),
                ),
                (
                    "selection_b",
                    CanvasView::selection(world, demo.auxiliary)
                        .map_err(|error| error.into_bevy_error())?
                        .len()
                        .to_string(),
                ),
                ("segment", segment),
                (
                    "draft",
                    draft
                        .map_or(0, |draft| draft.path.commands.len())
                        .to_string(),
                ),
            ],
        )
        .with_key_arg("mode", TextKey::Canvas(mode))
        .with_key_arg(
            "region",
            TextKey::Canvas(if demo.region.is_some() {
                CanvasKey::FixedRegion
            } else {
                CanvasKey::AutoRegion
            }),
        );
        bind(world, demo.readout, binding)?;
    }
    Ok(())
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result {
        let (root, demo) = world
            .query::<(Entity, &Demo)>()
            .single(world)
            .map(|(root, demo)| (root, demo.clone()))
            .map_err(|_| invalid("expected one Canvas demo"))?;
        match action {
            Action::Select => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Select)
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::Rectangle => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Rectangle(paint()))
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::Ellipse => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Ellipse(paint()))
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::Pen => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Pen(paint()))
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::Nodes => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::PathNodeEdit)
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::Add => add_rectangle(world, demo.document)?,
            Action::Raise | Action::Lower | Action::Front | Action::Back => {
                if let Some(shape) = CanvasView::single_selection(world, demo.main)
                    .map_err(|error| error.into_bevy_error())?
                {
                    let order = match action {
                        Action::Raise => CanvasOrder::Raise,
                        Action::Lower => CanvasOrder::Lower,
                        Action::Front => CanvasOrder::Front,
                        _ => CanvasOrder::Back,
                    };
                    CanvasDocument::reorder_shape(world, demo.document, shape, order)
                        .map_err(|error| error.into_bevy_error())?;
                } else {
                    bind(
                        world,
                        demo.result,
                        LocalizedText::new(TextKey::Canvas(CanvasKey::ChooseShape)),
                    )?;
                }
            }
            Action::Append
            | Action::Previous
            | Action::Next
            | Action::Insert
            | Action::Remove
            | Action::Line
            | Action::Quadratic
            | Action::Cubic
            | Action::Close => edit_path(world, root, &demo, action)?,
            Action::ZoomIn | Action::ZoomOut => {
                let zoom =
                    CanvasView::zoom(world, demo.main).map_err(|error| error.into_bevy_error())?;
                let factor = if matches!(action, Action::ZoomIn) {
                    1.25
                } else {
                    0.8
                };
                CanvasView::set_zoom_in_world(world, demo.main, (zoom * factor).clamp(0.1, 10.0))
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::Reset => {
                CanvasView::set_zoom_in_world(world, demo.main, 1.0)
                    .map_err(|error| error.into_bevy_error())?;
                CanvasView::set_center_in_world(world, demo.main, Vec2::ZERO)
                    .map_err(|error| error.into_bevy_error())?;
            }
            Action::ToggleAuxiliary => {
                let mut node = world
                    .get_mut::<Node>(demo.auxiliary)
                    .ok_or_else(|| invalid("auxiliary Node missing"))?;
                node.display = if node.display == Display::None {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            Action::Region => {
                world
                    .get_mut::<Demo>(root)
                    .ok_or_else(|| invalid("demo missing"))?
                    .region = if demo.region.is_some() {
                    None
                } else {
                    Some(Rect::from_corners(
                        Vec2::new(-100.0, -75.0),
                        Vec2::new(100.0, 75.0),
                    ))
                };
            }
            Action::Scale => {
                world
                    .get_mut::<Demo>(root)
                    .ok_or_else(|| invalid("demo missing"))?
                    .scale = if demo.scale == 1.0 {
                    2.0
                } else if demo.scale == 2.0 {
                    4.0
                } else {
                    1.0
                };
            }
            Action::Svg | Action::Png => export(world, &demo, action)?,
        }
        if !matches!(action, Action::Svg | Action::Png) {
            info!(?action, "执行 Canvas 演示操作");
        }
        refresh(world)
    });
}

fn export(world: &mut World, demo: &Demo, action: Action) -> Result {
    let format = if matches!(action, Action::Svg) {
        "svg"
    } else {
        "png"
    };
    let path = demo.directory.join(format!("canvas.{format}"));
    info!(format, ?path, region=?demo.region, scale=demo.scale, "请求导出 Canvas 文件");
    if matches!(action, Action::Png) && world.query::<&PendingExport>().iter(world).next().is_some()
    {
        info!(format, ?path, result = "busy", "Canvas 文件导出请求被拒绝");
        return bind(
            world,
            demo.result,
            LocalizedText::new(TextKey::Canvas(CanvasKey::Busy)),
        );
    }
    let result = (|| -> std::result::Result<_, CanvasExportError> {
        std::fs::create_dir_all(&demo.directory).map_err(|source| {
            CanvasExportError::Background(bevy_widgetry::job::JobFailure::from(
                crate::diagnostics::failure(
                    crate::diagnostics::GalleryError::Operation(source.into()),
                    bevy_widgetry::diagnostics::ErrorFrame::new("gallery.canvas.export_directory")
                        .field("path", demo.directory.display()),
                ),
            ))
        })?;
        if matches!(action, Action::Svg) {
            CanvasDocument::export_svg(world, demo.document, &path, demo.region)
                .map_err(CanvasExportError::Background)?;
            info!(format, ?path, result = "succeeded", "Canvas 文件导出结果");
            Ok(LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::Saved),
                vec![("path", path.display().to_string()), ("size", "SVG".into())],
            ))
        } else {
            let request = CanvasDocument::export_png(
                world,
                demo.document,
                &path,
                CanvasPngOptions {
                    region: demo.region,
                    scale: demo.scale,
                },
            )
            .map_err(CanvasExportError::Background)?;
            world.entity_mut(request).insert(PendingExport {
                result: demo.result,
            });
            info!(format, ?request, ?path, region=?demo.region, scale=demo.scale, result="submitted", "已提交 Canvas PNG 导出请求");
            Ok(LocalizedText::new(TextKey::Canvas(CanvasKey::Pending)))
        }
    })();
    let (binding, failure) = match result {
        Ok(binding) => (binding, None),
        Err(error) => {
            info!(format, ?path, result = "failed", "Canvas 文件导出结果");
            let binding = LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::Failed),
                vec![("error", error.to_string())],
            );
            (binding, Some(error))
        }
    };
    bind(world, demo.result, binding)?;
    match failure {
        Some(CanvasExportError::Job(error)) => {
            error.record_handled("停止 Canvas PNG 导出并显示失败状态", "导出请求被拒绝")
        }
        Some(CanvasExportError::Background(error)) => {
            error.record_handled("停止 Canvas 文件导出并显示失败状态", "导出失败")
        }
        _ => {}
    }
    Ok(())
}

fn png_finished(
    event: On<CanvasPngExportFinished>,
    pending: Query<&PendingExport>,
    mut commands: Commands,
) {
    let Ok(pending) = pending.get(event.entity) else {
        return;
    };
    let result = pending.result;
    let request = event.entity;
    let outcome = match &event.result {
        Ok(_) => "succeeded",
        Err(CanvasPngStop::Canceled) => "canceled",
        Err(CanvasPngStop::Failed(_)) => "failed",
    };
    info!(format="png", request=?event.entity, job_id=event.job_id.get(), path=?event.path, size=?event.result.as_ref().ok(), result=outcome, "Canvas 文件导出结果");
    let binding = match &event.result {
        Ok(size) => LocalizedText::with_args(
            TextKey::Canvas(CanvasKey::Saved),
            vec![
                ("path", event.path.display().to_string()),
                ("size", format!("{} × {}", size.x, size.y)),
            ],
        ),
        Err(error) => LocalizedText::with_args(
            TextKey::Canvas(CanvasKey::Failed),
            vec![("error", error.to_string())],
        ),
    };
    let failure = match &event.result {
        Err(bevy_widgetry::canvas::CanvasPngStop::Failed(error)) => Some(error.clone()),
        _ => None,
    };
    commands.queue(move |world: &mut World| -> Result {
        // An export may finish after the page that requested it has been removed.
        if world.get_entity(result).is_ok()
            && let Err(error) = bind(world, result, binding)
        {
            return Err(match failure {
                Some(failure) => failure.combine(error.into()).into_bevy_error(),
                None => error,
            });
        }
        if let Some(failure) = failure {
            failure.record_handled(
                "显示导出失败状态或放弃已关闭页面的导出",
                "终止本次 PNG 导出",
            );
        }
        debug!(
            ?request,
            page_present = world.get_entity(result).is_ok(),
            "Canvas PNG 导出结果已归档"
        );
        Ok(())
    });
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry::job::{JobFailure, JobManager};
    use bevy_widgetry_test_utils::LogCapture;

    fn fixture(directory: PathBuf) -> (App, Demo) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_observer(png_finished);
        let world = app.world_mut();
        let document = world.spawn(CanvasDocument).id();
        add_rectangle(world, document).unwrap();
        let result = world.spawn_empty().id();
        (
            app,
            Demo {
                document,
                main: result,
                auxiliary: result,
                readout: result,
                result,
                directory,
                region: None,
                scale: 2.0,
                segment: 0,
            },
        )
    }

    #[test]
    fn export_logs_sync_results_and_busy_without_submitting_another_request() {
        let directory = tempfile::tempdir().unwrap();
        let (mut app, mut demo) = fixture(directory.path().into());
        let logs = LogCapture::with_targets([
            "widget_gallery::pages::canvas",
            "widget_gallery",
            "bevy_widgetry",
        ]);
        logs.run(|| {
            export(app.world_mut(), &demo, Action::Svg).unwrap();
            assert!(demo.directory.join("canvas.svg").exists());
            let blocker = demo.directory.join("blocker");
            std::fs::write(&blocker, b"blocked").unwrap();
            demo.directory = blocker;
            export(app.world_mut(), &demo, Action::Svg).unwrap();
            app.world_mut().spawn(PendingExport {
                result: demo.result,
            });
            export(app.world_mut(), &demo, Action::Png).unwrap();
        });
        let records = logs.records();
        for outcome in ["succeeded", "failed", "busy"] {
            assert_eq!(
                records
                    .iter()
                    .filter(|r| r.matches_field("result", outcome))
                    .count(),
                1
            );
        }
        assert!(
            !records
                .iter()
                .any(|r| r.matches_field("result", "submitted"))
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.level == bevy::log::tracing::Level::ERROR)
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .query::<&PendingExport>()
                .iter(app.world())
                .count(),
            1
        );
    }

    #[test]
    fn png_terminal_logs_preserve_identity_and_handle_failure_after_page_removal() {
        let directory = tempfile::tempdir().unwrap();
        let (mut app, demo) = fixture(directory.path().into());
        let manager = JobManager::default();
        let (job, _) = manager
            .spawn_thread::<(), _>(Default::default(), |_| Ok(()))
            .unwrap();
        let logs = LogCapture::with_targets([
            "widget_gallery::pages::canvas",
            "widget_gallery",
            "bevy_widgetry",
        ]);
        logs.run(|| {
            for outcome in ["succeeded", "canceled", "failed"] {
                let request = app
                    .world_mut()
                    .spawn(PendingExport {
                        result: demo.result,
                    })
                    .id();
                let result = match outcome {
                    "succeeded" => Ok(UVec2::new(240, 160)),
                    "canceled" => Err(CanvasPngStop::Canceled),
                    _ => {
                        app.world_mut().despawn(demo.result);
                        Err(CanvasPngStop::Failed(JobFailure::external(
                            std::io::Error::other("write failed"),
                            bevy_widgetry::diagnostics::ErrorFrame::new("gallery.canvas.test"),
                        )))
                    }
                };
                app.world_mut().trigger(CanvasPngExportFinished {
                    entity: request,
                    job_id: job,
                    document: demo.document,
                    path: demo.directory.join("canvas.png"),
                    result,
                });
                app.world_mut().flush();
                assert_eq!(
                    logs.records()
                        .iter()
                        .filter(|r| r.matches_field("result", outcome)
                            && r.matches_field("request", &format!("{request:?}")))
                        .count(),
                    1
                );
            }
        });
        assert_eq!(
            logs.records()
                .iter()
                .filter(|r| r.level == bevy::log::tracing::Level::ERROR)
                .count(),
            1
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|r| r.matches_field("phase", "handled"))
                .count(),
            1
        );
    }
}
