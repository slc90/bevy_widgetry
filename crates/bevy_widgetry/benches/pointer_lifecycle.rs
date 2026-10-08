use bevy::{picking::pointer::*, prelude::*, ui::Pressed, ui_widgets::ScrollbarDragState};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    list_view::{
        WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
        WidgetryListViewPlugin, WidgetryListViewRenderer,
    },
    scroll_area::{ScrollAxis, WidgetryScrollArea, WidgetryScrollAreaPlugin},
    table::*,
};
use bevy_widgetry_test_utils::{
    benchmark::{Harness, missing, run},
    picking_app, pointer_ids, queue_pointer, spawn_picking_camera,
};

#[derive(Resource, Default)]
struct ResizeEvents {
    starts: usize,
    cancels: usize,
}

struct Fixture {
    app: App,
    target: Entity,
    location: Location,
    kind: &'static str,
}

fn fixture(count: usize, kind: &'static str) -> Result<Fixture> {
    let mut app = picking_app();
    app.init_resource::<ResizeEvents>().add_observer(
        |event: On<WidgetryTableEvent>, mut seen: ResMut<ResizeEvents>| match event.kind {
            WidgetryTableEventKind::ColumnResizeStart(_) => seen.starts += 1,
            WidgetryTableEventKind::ColumnResizeCancel(_) => seen.cancels += 1,
            _ => {}
        },
    );
    app.add_plugins((
        WidgetryButtonPlugin,
        WidgetryScrollAreaPlugin,
        WidgetryListViewPlugin,
    ));
    app.register_widgetry_list_view::<u32>()
        .map_err(|error| BevyError::error(error.to_string()))?;
    app.register_widgetry_table::<u32>();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|_: &u32| {
        bsn_list![
            (Node {
                width: px(20),
                height: px(20)
            })
        ]
    }))
    .map_err(|error| BevyError::error(error.to_string()))?;
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|_: &u32| {
        bsn_list![
            (Node {
                width: px(20),
                height: px(20)
            })
        ]
    }))
    .map_err(|error| BevyError::error(error.to_string()))?;
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: (400, 400).into(),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ))
        .id();
    let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
    let root=match kind {
        "button"=>{
            for _ in 1..count {app.world_mut().spawn_scene(bsn!{@WidgetryButton Node {position_type:PositionType::Absolute,left:px(300),top:px(200),width:px(20),height:px(20)}}).map_err(|error|BevyError::error(error.to_string()))?;}
            app.world_mut().spawn_scene(bsn!{@WidgetryButton Node {width:px(100),height:px(60)} template(move |_|Ok(UiTargetCamera(camera)))}).map_err(|error|BevyError::error(error.to_string()))?.id()
        },
        "list"=>{
            let mut model=WidgetryListModel::default();for index in 0..count {model.push(index as u32).map_err(|error|BevyError::error(error.to_string()))?;}
            let source=app.world_mut().spawn(model).id();
            app.world_mut().spawn_scene(bsn!{@WidgetryListView::<u32> {@source:source,@item_height:20.0,@renderer:{WidgetryListViewRenderer::new(|_,_:&u32|bsn_list![(Node {height:px(20),width:px(100)})])}} Node {width:px(180),height:px(100)} template(move |_|Ok(UiTargetCamera(camera)))}).map_err(|error|BevyError::error(error.to_string()))?.id()
        },
        "table"=>{
            let mut model=WidgetryTableModel::default();for index in 0..count {model.push_row(index as u32).map_err(|error|BevyError::error(error.to_string()))?;}
            model.push_column(WidgetryTableColumn::new(WidgetryTableHeaderValue::new(0u32),0u32,|row:&u32,_:&u32|WidgetryTableCellValue::new(*row))).map_err(|error|BevyError::error(error.to_string()))?;
            let source=app.world_mut().spawn(model).id();
            app.world_mut().spawn_scene(bsn!{@WidgetryTable::<u32> {@source:source} Node {width:px(180),height:px(100)} template(move |_|Ok(UiTargetCamera(camera)))}).map_err(|error|BevyError::error(error.to_string()))?.id()
        },
        _=>app.world_mut().spawn_scene(bsn!{@WidgetryScrollArea {@axis:ScrollAxis::Vertical,@children:bsn_list![(Node {width:px(120),height:px(count as f32*20.0),flex_shrink:0.0})]} Node {width:px(150),height:px(100)} template(move |_|Ok(UiTargetCamera(camera)))}).map_err(|error|BevyError::error(error.to_string()))?.id(),
    };
    let id = pointer_ids()[1];
    app.world_mut().spawn(id);
    for _ in 0..4 {
        app.update();
    }
    let target = match kind {
        "list" => app
            .world_mut()
            .query::<(Entity, &WidgetryListViewItem)>()
            .iter(app.world())
            .find(|(_, item)| item.index == 0)
            .map(|(entity, _)| entity)
            .ok_or_else(|| missing("visible ListView item"))?,
        "table" => {
            let header = app
                .world_mut()
                .query_filtered::<Entity, With<WidgetryTableColumnHeader>>()
                .iter(app.world())
                .next()
                .ok_or_else(|| missing("Table header"))?;
            *app.world()
                .get::<Children>(header)
                .and_then(|children| children.last())
                .ok_or_else(|| missing("resize handle"))?
        }
        "scroll" => app
            .world_mut()
            .query_filtered::<Entity, With<bevy::ui_widgets::ScrollbarThumb>>()
            .iter(app.world())
            .next()
            .ok_or_else(|| missing("scroll thumb"))?,
        _ => root,
    };
    let location = Location {
        target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
            .normalize(None)
            .ok_or_else(|| missing("window target"))?,
        position: Vec2::new(30.0, 15.0),
    };
    let mut fixture = Fixture {
        app,
        target,
        location,
        kind,
    };
    cycle(&mut fixture, false)?;
    Ok(fixture)
}

fn input(fixture: &mut Fixture, action: PointerAction) {
    queue_pointer(
        &mut fixture.app,
        pointer_ids()[1],
        fixture.location.clone(),
        action,
    );
    fixture.app.update();
}

fn cycle(fixture: &mut Fixture, invalid: bool) -> Result {
    let starts = fixture.app.world().resource::<ResizeEvents>().starts;
    let cancels = fixture.app.world().resource::<ResizeEvents>().cancels;
    if fixture.kind == "table" || fixture.kind == "scroll" {
        fixture.location.position = fixture
            .app
            .world()
            .get::<UiGlobalTransform>(fixture.target)
            .ok_or_else(|| missing("drag geometry"))?
            .translation;
    } else {
        fixture.location.position = Vec2::new(30.0, 15.0);
    }
    input(fixture, PointerAction::Move { delta: Vec2::ZERO });
    input(fixture, PointerAction::Press(PointerButton::Primary));
    if (fixture.kind == "button" || fixture.kind == "list")
        && fixture.app.world().get::<Pressed>(fixture.target).is_none()
    {
        return Err(missing("Pointer Press must reach the control"));
    }
    if fixture.kind == "table" || fixture.kind == "scroll" {
        let delta = if fixture.kind == "table" {
            Vec2::new(20.0, 0.0)
        } else {
            Vec2::new(0.0, 20.0)
        };
        fixture.location.position += delta;
        input(fixture, PointerAction::Move { delta });
        if fixture.kind == "scroll"
            && fixture
                .app
                .world()
                .get::<ScrollbarDragState>(fixture.target)
                .is_none_or(|drag| !drag.dragging)
        {
            return Err(missing("Pointer drag must start on the thumb"));
        }
        fixture.location.position -= delta;
        input(fixture, PointerAction::Move { delta: -delta });
    }
    if invalid {
        let pointer = fixture
            .app
            .world_mut()
            .query::<(Entity, &PointerId)>()
            .iter(fixture.app.world())
            .find(|(_, id)| **id == pointer_ids()[1])
            .map(|(entity, _)| entity)
            .ok_or_else(|| missing("active pointer"))?;
        fixture
            .app
            .world_mut()
            .get_mut::<PointerLocation>(pointer)
            .ok_or_else(|| missing("active location"))?
            .location = None;
        fixture.app.update();
        if fixture.app.world().get::<Pressed>(fixture.target).is_some()
            || fixture
                .app
                .world()
                .get::<ScrollbarDragState>(fixture.target)
                .is_some_and(|drag| drag.dragging)
            || (fixture.kind == "table"
                && fixture.app.world().resource::<ResizeEvents>().cancels != cancels + 1)
        {
            return Err(missing(
                "invalid source must terminate before producer cleanup",
            ));
        }
        // 位置失效不会重置 Bevy 的 input gesture map。
        // 新一轮复用 Pointer 前按 producer 契约取消旧输入，计时包含这次 cleanup update。
        input(fixture, PointerAction::Cancel);
    } else {
        input(fixture, PointerAction::Cancel);
    }
    if fixture.kind == "table" {
        let seen = fixture.app.world().resource::<ResizeEvents>();
        if seen.starts != starts + 1 || seen.cancels != cancels + 1 {
            return Err(missing("Table drag must have one Start and Cancel"));
        }
    }
    Ok(())
}

fn main() -> Result {
    let mut harness = Harness::new("ordinary-pointer-criterion")?;
    for count in [10, 1000] {
        for kind in ["button", "list", "table", "scroll"] {
            for action in ["idle", "cancel", "invalid"] {
                run(
                    &mut harness,
                    &format!("ordinary-pointer/{kind}/n{count}/{action}"),
                    false,
                    || fixture(count, kind),
                    |fixture, _| {
                        if action == "idle" {
                            fixture.app.update();
                            Ok(())
                        } else {
                            cycle(fixture, action == "invalid")
                        }
                    },
                    |fixture| {
                        if fixture.app.world().get::<Pressed>(fixture.target).is_some()
                            || fixture
                                .app
                                .world()
                                .get::<ScrollbarDragState>(fixture.target)
                                .is_some_and(|drag| drag.dragging)
                        {
                            return Err(missing("terminated Pointer state"));
                        }
                        Ok(fixture.app.world().entities().len())
                    },
                )?;
            }
        }
    }
    harness.finish()
}
