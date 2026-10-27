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
            .add_observer(png_finished)
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
    Add,
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

fn add_rectangle(world: &mut World, document: Entity) -> Result {
    CanvasDocument::create_rectangle(
        world,
        document,
        CanvasRectangle {
            corner_radius: 0.0,
            size: Vec2::new(120.0, 80.0),
        },
        Transform::default(),
        paint(),
    )?;
    Ok(())
}

fn enter(world: &mut World) -> Result {
    let directory = std::env::current_exe()
        .map_err(|error| {
            error!(%error, "无法确定 Canvas 导出目录");
            BevyError::error(error)
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
        )?;
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
        )?;
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
        )?;
        CanvasDocument::append_rectangle(
            world,
            document,
            path,
            Rect::new(-16.0, -10.0, 10.0, 10.0),
        )?;
        CanvasDocument::append_circle(world, document, path, Vec2::new(-55.0, 75.0), 8.0)?;
        CanvasDocument::append_ellipse(
            world,
            document,
            path,
            Vec2::new(-25.0, 75.0),
            Vec2::new(12.0, 6.0),
        )?;
        CanvasDocument::append_line(
            world,
            document,
            path,
            Vec2::new(0.0, 75.0),
            Vec2::new(15.0, 75.0),
        )?;
        CanvasDocument::append_polyline(
            world,
            document,
            path,
            &[
                Vec2::new(25.0, 80.0),
                Vec2::new(33.0, 65.0),
                Vec2::new(40.0, 80.0),
            ],
        )?;
        CanvasDocument::append_polygon(
            world,
            document,
            path,
            &[
                Vec2::new(50.0, 80.0),
                Vec2::new(58.0, 65.0),
                Vec2::new(66.0, 80.0),
            ],
        )?;
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
            directory:directory, region:None, scale:1.0 }
        Node { width:percent(100), flex_direction:FlexDirection::Column, row_gap:px(12), flex_shrink:0.0 }
        Children [
            @localized(TextKey::Canvas(CanvasKey::Description)) WidgetryText Node {max_width:percent(100)}--
            Node {column_gap:px(8),row_gap:px(8),flex_wrap:FlexWrap::Wrap,max_width:percent(100)} Children [{controls}]--
            #Readout Name("CanvasReadout") Text("") WidgetryText Node {max_width:percent(100)}--
            Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(12)}
            Children [
                Node {width:percent(100),min_width:px(0),flex_shrink:0.0,row_gap:px(4),flex_direction:FlexDirection::Column}
                Children [
                    @localized(TextKey::Canvas(CanvasKey::Main)) WidgetryText--
                    #Main Name("CanvasMain") @CanvasView {@document:{Some(document)}}
                    Node {width:percent(100),height:px(320),flex_shrink:0.0}
                    BackgroundColor(Color::srgb(0.09,0.11,0.16))
                ]--
                Node {width:percent(100),min_width:px(0),flex_shrink:0.0,row_gap:px(4),flex_direction:FlexDirection::Column}
                Children [
                    @localized(TextKey::Canvas(CanvasKey::Secondary)) WidgetryText--
                    #Auxiliary Name("CanvasAuxiliary") @CanvasView {@document:{Some(document)},@zoom:0.5,@center:Vec2::new(20.0,-10.0)}
                    Node {width:percent(100),height:px(220),flex_shrink:0.0}
                    BackgroundColor(Color::srgb(0.13,0.1,0.16))
                ]
            ]--
            @localized_args(TextKey::Canvas(CanvasKey::Paths),vec![("path",path)]) WidgetryText Node {max_width:percent(100)}--
            #Result Name("CanvasExportResult") @localized(TextKey::Canvas(CanvasKey::Ready)) WidgetryText Node {max_width:percent(100)}
        ]
    }
}

fn invalid(reason: &str) -> BevyError {
    error!(reason, "Canvas 演示内部结构失效");
    BevyError::error(reason.to_owned())
}

fn bind(world: &mut World, entity: Entity, binding: LocalizedText) -> Result {
    let mut label = world
        .get_entity_mut(entity)
        .map_err(|_| invalid("Canvas label missing"))?;
    label.insert(binding);
    Ok(())
}

fn refresh(world: &mut World) -> Result {
    let demos: Vec<_> = world.query::<&Demo>().iter(world).cloned().collect();
    for demo in demos {
        let a = CanvasView::center(world, demo.main)?;
        let b = CanvasView::center(world, demo.auxiliary)?;
        let mode = match CanvasView::mode(world, demo.main)? {
            CanvasMode::Select => CanvasKey::Select,
            CanvasMode::Rectangle(_) => CanvasKey::Rectangle,
            CanvasMode::Ellipse(_) => CanvasKey::Ellipse,
        };
        let binding = LocalizedText::with_args(
            TextKey::Canvas(CanvasKey::Readout),
            vec![
                (
                    "zoom_a",
                    format!("{:.2}", CanvasView::zoom(world, demo.main)?),
                ),
                (
                    "zoom_b",
                    format!("{:.2}", CanvasView::zoom(world, demo.auxiliary)?),
                ),
                ("center_a", format!("{:.1}, {:.1}", a.x, a.y)),
                ("center_b", format!("{:.1}, {:.1}", b.x, b.y)),
                ("scale", demo.scale.to_string()),
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
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Select)?;
            }
            Action::Rectangle => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Rectangle(paint()))?;
            }
            Action::Ellipse => {
                CanvasView::set_mode_in_world(world, demo.main, CanvasMode::Ellipse(paint()))?;
            }
            Action::Add => add_rectangle(world, demo.document)?,
            Action::ZoomIn | Action::ZoomOut => {
                let zoom = CanvasView::zoom(world, demo.main)?;
                let factor = if matches!(action, Action::ZoomIn) {
                    1.25
                } else {
                    0.8
                };
                CanvasView::set_zoom_in_world(world, demo.main, (zoom * factor).clamp(0.1, 10.0))?;
            }
            Action::Reset => {
                CanvasView::set_zoom_in_world(world, demo.main, 1.0)?;
                CanvasView::set_center_in_world(world, demo.main, Vec2::ZERO)?;
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
        info!(?action, "执行 Canvas 演示操作");
        refresh(world)
    });
}

fn export(world: &mut World, demo: &Demo, action: Action) -> Result {
    if matches!(action, Action::Png) && world.query::<&PendingExport>().iter(world).next().is_some()
    {
        return bind(
            world,
            demo.result,
            LocalizedText::new(TextKey::Canvas(CanvasKey::Busy)),
        );
    }
    let result = (|| -> std::result::Result<_, CanvasExportError> {
        std::fs::create_dir_all(&demo.directory).map_err(CanvasExportError::Io)?;
        if matches!(action, Action::Svg) {
            let path = demo.directory.join("canvas.svg");
            CanvasDocument::export_svg(world, demo.document, &path, demo.region)?;
            info!(?path, "Canvas SVG 文件导出完成");
            Ok(LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::Saved),
                vec![("path", path.display().to_string()), ("size", "SVG".into())],
            ))
        } else {
            let path = demo.directory.join("canvas.png");
            let request = CanvasDocument::export_png(
                world,
                demo.document,
                &path,
                CanvasPngOptions {
                    region: demo.region,
                    scale: demo.scale,
                },
            )?;
            world.entity_mut(request).insert(PendingExport {
                result: demo.result,
            });
            info!(?request, ?path, "已发起 Canvas PNG 导出");
            Ok(LocalizedText::new(TextKey::Canvas(CanvasKey::Pending)))
        }
    })();
    let binding = match result {
        Ok(binding) => binding,
        Err(error) => {
            warn!(%error,"Canvas 文件导出失败");
            LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::Failed),
                vec![("error", error.to_string())],
            )
        }
    };
    bind(world, demo.result, binding)
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
    let binding = match &event.result {
        Ok(size) => {
            info!(request=?event.entity,path=?event.path,?size,"Canvas PNG 文件导出完成");
            LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::Saved),
                vec![
                    ("path", event.path.display().to_string()),
                    ("size", format!("{} × {}", size.x, size.y)),
                ],
            )
        }
        Err(error) => {
            warn!(request=?event.entity,path=?event.path,%error,"Canvas PNG 文件导出失败");
            LocalizedText::with_args(
                TextKey::Canvas(CanvasKey::Failed),
                vec![("error", error.to_string())],
            )
        }
    };
    commands.queue(move |world: &mut World| -> Result {
        // An export may finish after the page that requested it has been removed.
        if world.get_entity(result).is_ok() {
            bind(world, result, binding)?;
        }
        Ok(())
    });
}
