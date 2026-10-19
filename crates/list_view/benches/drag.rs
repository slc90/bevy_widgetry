use bevy::picking::events::{PointerDragDrop, PointerDragOver, PointerDragStart};
use bevy::prelude::*;
use bevy_widgetry_list_view::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app};
use bevy_widgetry_test_utils::{primary_drag_end, primary_press};

struct Fixture {
    app: App,
    source: Entity,
    root: Entity,
    first: WidgetryListItemId,
    second: WidgetryListItemId,
    expected: WidgetryListItemId,
}

fn main() -> Result {
    let mut harness = Harness::new("list-drag")?;
    for count in [1_000, 10_000, 100_000] {
        run(
            &mut harness,
            &format!("list_drag/n{count}/move_and_update"),
            false,
            || {
                let mut app = ui_app()?;
                app.add_plugins(WidgetryListViewPlugin);
                app.register_widgetry_list_view::<u32>()?;
                let mut model = WidgetryListModel::<u32>::default();
                for index in 0..count {
                    model.push(index)?;
                }
                let first = model.id(0).ok_or_else(|| missing("first ID"))?;
                let second = model.id(1).ok_or_else(|| missing("second ID"))?;
                let source = app.world_mut().spawn(model).id();
                let root = app.world_mut().spawn_scene(bsn! {
                    @WidgetryListView::<u32> {
                        @source: source, @draggable: true,
                        @renderer: {WidgetryListViewRenderer::new(|value: &u32| bsn_list! { Text({value.to_string()}) bevy_widgetry_core::text::WidgetryText })},
                    }
                    Node { width: px(640), height: px(384) }
                }).map_err(BevyError::error)?.id();
                settle(&mut app);
                Ok(Fixture {
                    app,
                    source,
                    root,
                    first,
                    second,
                    expected: first,
                })
            },
            |fixture, _| {
                let world = fixture.app.world_mut();
                let rows = world
                    .query::<(Entity, &WidgetryListViewItem, &UiGlobalTransform)>()
                    .iter(world)
                    .filter(|(_, item, _)| item.index < 2)
                    .map(|(entity, item, transform)| (item.index, entity, transform.translation))
                    .collect::<Vec<_>>();
                let from = rows
                    .iter()
                    .find(|r| r.0 == 0)
                    .ok_or_else(|| missing(&format!("first row; materialized={rows:?}")))?
                    .1;
                let (_, to, center) = *rows
                    .iter()
                    .find(|r| r.0 == 1)
                    .ok_or_else(|| missing("second row"))?;
                let camera = world
                    .query_filtered::<Entity, With<Camera>>()
                    .single(world)
                    .map_err(BevyError::error)?;
                let press = primary_press(from);
                world.trigger(press.clone());
                world.flush();
                world.trigger(PointerDragStart {
                    entity: from,
                    pointer: press.pointer.clone(),
                    button: press.button,
                    hit: press.hit,
                });
                world.flush();
                let mut drop = primary_press(to);
                drop.hit.camera = camera;
                drop.pointer.position = center + Vec2::new(0.0, 10.0);
                world.trigger(PointerDragOver {
                    entity: to,
                    pointer: drop.pointer.clone(),
                    button: drop.button,
                    dragged: from,
                    hit: drop.hit.clone(),
                });
                world.flush();
                world.trigger(PointerDragDrop {
                    entity: to,
                    pointer: drop.pointer,
                    button: drop.button,
                    dropped: from,
                    hit: drop.hit,
                });
                world.flush();
                world.trigger(primary_drag_end(from));
                world.flush();
                fixture.expected = if fixture.expected == fixture.first {
                    fixture.second
                } else {
                    fixture.first
                };
                fixture.app.update();
                Ok(())
            },
            |fixture| {
                let world = fixture.app.world_mut();
                let projection = world
                    .get::<WidgetryListProjection>(fixture.root)
                    .ok_or_else(|| missing("projection"))?;
                let model = world
                    .get::<WidgetryListModel<u32>>(fixture.source)
                    .ok_or_else(|| missing("model"))?;
                if projection.id(0) != Some(fixture.expected)
                    || model.id(0) != Some(fixture.first)
                    || model.len() != count as usize
                {
                    return Err(missing("actual View move with unchanged Model"));
                }
                let rows = world.query::<&WidgetryListViewItem>().iter(world).count();
                if rows > 20 {
                    return Err(missing("bounded visible rows"));
                }
                Ok(world.entities().count_spawned())
            },
        )?;
    }
    harness.finish()
}
