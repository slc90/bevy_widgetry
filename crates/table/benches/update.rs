//! Table 两轴规模与 viewport 独立变化；测量 CPU/ECS/UI，不包含 GPU 或真实输入 latency。

use bevy::camera::NormalizedRenderTarget;
use bevy::picking::{
    backend::HitData,
    events::{Drag, DragEnd, DragStart, Pointer},
    pointer::{Location, PointerButton, PointerId},
};
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_table::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};

/// 一个 source/view 配对，scroll 直接修改公开 Component；没有伪造 viewport cache。
struct Fixture {
    app: App,
    source: Entity,
    root: Entity,
    body: Entity,
    /// 通过公开 Header hierarchy 找到的真实 resize strip。
    handle: Entity,
}

/// 分别改变 Row、Column、viewport 与 renderer 内容复杂度，不展开无意义笛卡尔积。
fn main() -> Result {
    let mut harness = Harness::new("table-criterion")?;
    for (rows, columns, width, height, rich) in [
        (1_000, 20, 646, 368, false),
        (10_000, 20, 646, 368, false),
        (100_000, 20, 646, 368, false),
        (10_000, 200, 646, 368, false),
        (10_000, 2_000, 646, 368, false),
        (10_000, 20, 1246, 704, false),
        (10_000, 20, 646, 368, true),
    ] {
        let prefix = format!("table/r{rows}_c{columns}/v{width}x{height}/rich{rich}");
        for action in [
            "first_scene",
            "idle",
            "scroll_x",
            "scroll_y",
            "scroll_xy",
            "visible_mutation",
            "offscreen_mutation",
            "resize",
            "column_drag",
            "hidden_idle",
            "rebuild",
            "destroy",
        ] {
            let name = format!("{prefix}/{action}");
            run(
                &mut harness,
                &name,
                matches!(action, "first_scene" | "destroy"),
                || {
                    let mut fixture = fixture(rows, columns, rich)?;
                    if action != "first_scene" {
                        spawn_view(&mut fixture, width, height)?;
                        if action == "hidden_idle" {
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("root Node"))?
                                .display = Display::None;
                        }
                        settle(&mut fixture.app);
                        validate(&mut fixture, action == "hidden_idle")?;
                        if action == "column_drag" {
                            fixture.handle = resize_handle(&mut fixture.app, fixture.source)?;
                        }
                    }
                    Ok(fixture)
                },
                |fixture, index| {
                    let app = &mut fixture.app;
                    match action {
                        "column_drag" => {
                            let target = fixture.handle;
                            let distance =
                                Vec2::new(if index.is_multiple_of(2) { 20.0 } else { -20.0 }, 0.0);
                            app.world_mut().trigger(pointer(
                                target,
                                DragStart {
                                    button: PointerButton::Primary,
                                    hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                                },
                            ));
                            app.world_mut().flush();
                            app.world_mut().trigger(pointer(
                                target,
                                Drag {
                                    button: PointerButton::Primary,
                                    distance,
                                    delta: distance,
                                },
                            ));
                            app.world_mut().flush();
                            app.world_mut().trigger(pointer(
                                target,
                                DragEnd {
                                    button: PointerButton::Primary,
                                    distance,
                                },
                            ));
                            app.world_mut().flush();
                            settle(app);
                        }
                        "first_scene" => {
                            spawn_view(fixture, width, height)?;
                            settle(&mut fixture.app);
                        }
                        "scroll_x" | "scroll_y" | "scroll_xy" => {
                            let offset = if index.is_multiple_of(2) { 120.0 } else { 0.0 };
                            app.world_mut()
                                .get_mut::<ScrollPosition>(fixture.body)
                                .ok_or_else(|| missing("ScrollPosition"))?
                                .0 = Vec2::new(
                                if action != "scroll_y" { offset } else { 0.0 },
                                if action != "scroll_x" { offset } else { 0.0 },
                            );
                            app.update();
                        }
                        "visible_mutation" | "offscreen_mutation" => {
                            let row = if action == "visible_mutation" {
                                0
                            } else {
                                rows - 1
                            };
                            {
                                let mut model = app
                                    .world_mut()
                                    .get_mut::<WidgetryTableModel<u32>>(fixture.source)
                                    .ok_or_else(|| missing("TableModel"))?;
                                *model.row_mut(row)?.ok_or_else(|| missing("row"))? = index as u32;
                            }
                            app.update();
                        }
                        "resize" => {
                            app.world_mut()
                                .get_mut::<Node>(fixture.root)
                                .ok_or_else(|| missing("root Node"))?
                                .width =
                                px(width as f32
                                    + if index.is_multiple_of(2) { 120.0 } else { 0.0 });
                            settle(app);
                        }
                        "rebuild" => {
                            app.world_mut().despawn(fixture.root);
                            spawn_view(fixture, width, height)?;
                            settle(&mut fixture.app);
                        }
                        "destroy" => {
                            app.world_mut().despawn(fixture.root);
                            app.world_mut().despawn(fixture.source);
                            app.update();
                        }
                        _ => app.update(),
                    }
                    Ok(())
                },
                |fixture| {
                    if fixture.root != Entity::PLACEHOLDER && action != "destroy" {
                        validate(fixture, action == "hidden_idle")?;
                    }
                    Ok(fixture.app.world().entities().count_spawned())
                },
            )?;
        }
    }
    harness.finish()
}

/// Model 与 plugin 装配在计时外；first_scene 单独计时 BSN 展开与三个 UI update。
fn fixture(rows: usize, columns: u32, rich: bool) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.register_widgetry_table::<u32>();
    app.register_table_cell_renderer(WidgetryTableCellRenderer::new(move |value: &String| {
        bsn_list![(Node { flex_direction: FlexDirection::Column } Children [Text({value.clone()}), {rich.then(|| bsn! { Text("details: 123.45 / active") })}])]
    }))?;
    app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &String| {
        bsn_list![Text({ value.clone() })]
    }))?;
    let mut model = WidgetryTableModel::default();
    for row in 0..rows {
        model.push_row(row as u32)?;
    }
    for column in 0..columns {
        model.push_column(WidgetryTableColumn::new(
            WidgetryTableHeaderValue::new(format!("Column {column}")),
            column,
            |row: &u32, column: &u32| WidgetryTableCellValue::new(format!("{row}/{column}")),
        ))?;
    }
    let source = app.world_mut().spawn(model).id();
    Ok(Fixture {
        app,
        source,
        root: Entity::PLACEHOLDER,
        body: Entity::PLACEHOLDER,
        handle: Entity::PLACEHOLDER,
    })
}

/// 通过公开 physical Header 与 Children/Node 找第一列 strip，不访问私有 session。
fn resize_handle(app: &mut App, source: Entity) -> Result<Entity> {
    let world = app.world_mut();
    let column = world
        .get::<WidgetryTableModel<u32>>(source)
        .and_then(|model| model.column_id(0))
        .ok_or_else(|| missing("first Column"))?;
    let header = world
        .query::<(Entity, &WidgetryTableColumnHeader)>()
        .iter(world)
        .find(|(_, header)| header.column == column)
        .map(|(entity, _)| entity)
        .ok_or_else(|| missing("Column Header"))?;
    world
        .get::<Children>(header)
        .into_iter()
        .flat_map(|children| children.iter())
        .find(|&entity| {
            world.get::<Node>(entity).is_some_and(|node| {
                node.position_type == PositionType::Absolute
                    && node.right == px(0)
                    && node.width == px(6)
            })
        })
        .ok_or_else(|| missing("resize strip"))
}

/// 驱动生产 Pointer observer/command 路径；CPU 计时不包含 OS picking 或 GPU presentation。
fn pointer<E: Clone + Reflect + std::fmt::Debug>(target: Entity, event: E) -> Pointer<E> {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 1246,
                height: 704,
            },
            position: Vec2::ZERO,
        },
        event,
        target,
    )
}

/// 通过真实 BSN 创建 view，并读取公开 Body marker。
fn spawn_view(fixture: &mut Fixture, width: u32, height: u32) -> Result {
    let source = fixture.source;
    fixture.root = fixture.app.world_mut().spawn_scene(bsn! { @WidgetryTable::<u32> { @source: source } Node { width: px(width as f32), height: px(height as f32) } })?.id();
    fixture.body = fixture
        .app
        .world()
        .get::<Children>(fixture.root)
        .ok_or_else(|| missing("Table Children"))?
        .iter()
        .find(|&entity| {
            fixture
                .app
                .world()
                .get::<WidgetryTableBody>(entity)
                .is_some()
        })
        .ok_or_else(|| missing("TableBody"))?;
    Ok(())
}

/// 在计时外拒绝空 projection，hidden 用例应完全回收可见 Cell。
fn validate(fixture: &mut Fixture, hidden: bool) -> Result {
    let world = fixture.app.world_mut();
    let cells = world.query::<&WidgetryTableCell>().iter(world).count();
    if (cells == 0) != hidden {
        return Err(BevyError::error(
            "Table benchmark projection readiness failed",
        ));
    }
    if !hidden {
        validate_text(&mut fixture.app)?;
    }
    Ok(())
}
