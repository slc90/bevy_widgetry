//! State：无选择/有效选择、model identity/index/revision 与 Field subtree；stimuli 为 public API、authority/CRUD。
//! Invariant：shell identity 保持、旧内容当帧清理、投影不发变化通知；asset readiness 与动态生成消费帧分别验证。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::camera::visibility::VisibilitySystems;
use bevy::ecs::world::CommandQueue;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, Button, ValueChange};
use bevy_widgetry_asset::BuiltinFont;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, advance_until, scene_app, spawn_ui_camera,
};
use std::time::Duration;

#[derive(Resource, Default)]
struct Changes(Vec<Option<WidgetryListItemId>>);

fn record(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    roots: Query<(), With<WidgetryComboBox<String>>>,
    mut changes: ResMut<Changes>,
) {
    if roots.contains(event.source) {
        changes.0.push(event.value);
    }
}

fn app() -> App {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<String>()
        .unwrap()
        .init_resource::<Changes>()
        .add_observer(record);
    app
}

fn combo(app: &mut App, source: Entity) -> Entity {
    app.world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox::<String> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|index, item: &String| {
                    bsn_list![(Node Children [(Text({format!("{index}:{item}")}))])]
                })},
            }
        })
        .unwrap()
        .id()
}

fn list(world: &World, root: Entity) -> Entity {
    let popup = world.get::<Children>(root).unwrap()[1];
    world.get::<Children>(popup).unwrap()[0]
}

fn content(world: &World, root: Entity) -> Entity {
    let field = world.get::<Children>(root).unwrap()[0];
    world.get::<Children>(field).unwrap()[0]
}

fn rendered(world: &World, root: Entity) -> (Entity, Entity, &str) {
    let wrapper = world.get::<Children>(content(world, root)).unwrap()[0];
    let text = world.get::<Children>(wrapper).unwrap()[0];
    (wrapper, text, &world.get::<Text>(text).unwrap().0)
}

#[derive(Resource, Default)]
struct CommittedChanges(Vec<Option<WidgetryListItemId>>);

#[derive(Resource, Default)]
struct ReentrantChanges(Vec<(Option<WidgetryListItemId>, Option<WidgetryListItemId>)>);

#[test]
fn reentrant_programmatic_selection_preserves_root_notification_order() {
    for (register_first, clear) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut app = scene_app();
        app.init_resource::<ReentrantChanges>()
            .init_resource::<InputFocus>();
        let mut model = WidgetryListModel::default();
        let a = model.push(String::from("A")).unwrap();
        let b = model.push(String::from("B")).unwrap();
        let source = app.world_mut().spawn(model).id();
        if register_first {
            app.register_widgetry_combo_box::<String>().unwrap();
        }
        app.add_observer(
            move |event: On<ValueChange<Option<WidgetryListItemId>>>,
                  lists: Query<&ChildOf, With<WidgetryListView<String>>>,
                  parents: Query<&ChildOf>,
                  mut commands: Commands| {
                if event.value != Some(b) {
                    return;
                }
                let Ok(parent) = lists.get(event.source) else {
                    return;
                };
                let root = parents.get(parent.parent()).unwrap().parent();
                if clear {
                    WidgetryComboBox::<String>::clear_selection(&mut commands, root);
                } else {
                    WidgetryComboBox::<String>::set_selected(&mut commands, root, a);
                }
            },
        );
        app.register_widgetry_combo_box::<String>().unwrap();
        app.add_observer(
            move |event: On<ValueChange<Option<WidgetryListItemId>>>,
                  roots: Query<&Children, With<WidgetryComboBox<String>>>,
                  children: Query<&Children>,
                  states: Query<&WidgetryListViewState>,
                  mut changes: ResMut<ReentrantChanges>,
                  mut commands: Commands| {
                if let Ok(root_children) = roots.get(event.source) {
                    let list = children.get(root_children[1]).unwrap()[0];
                    changes
                        .0
                        .push((event.value, states.get(list).unwrap().selected));
                    if event.value == Some(b) {
                        commands.queue(move |world: &mut World| {
                            assert!(
                                world
                                    .get_mut::<WidgetryListModel<String>>(source)
                                    .unwrap()
                                    .move_item(0, 1)
                            );
                        });
                    }
                }
            },
        );
        let root = combo(&mut app, source);
        app.update();
        let field = app.world().get::<Children>(root).unwrap()[0];
        let popup = app.world().get::<Children>(root).unwrap()[1];
        let list = list(app.world(), root);
        app.world_mut().trigger(Activate { entity: field });
        WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, b);
        app.world_mut().flush();
        let next = (!clear).then_some(a);
        assert_eq!(
            app.world().resource::<ReentrantChanges>().0,
            vec![(Some(b), Some(b)), (next, next)]
        );
        assert_eq!(
            *app.world().get::<WidgetryListViewState>(list).unwrap(),
            WidgetryListViewState {
                selected: next,
                active: Some(if clear { b } else { a }),
            }
        );
        assert_eq!(
            *app.world().get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    }
}

#[test]
fn programmatic_root_notification_reads_committed_authority_and_keeps_popup() {
    let mut app = app();
    app.init_resource::<CommittedChanges>()
        .init_resource::<InputFocus>();
    app.add_observer(
        |event: On<ValueChange<Option<WidgetryListItemId>>>,
         roots: Query<&Children, With<WidgetryComboBox<String>>>,
         children: Query<&Children>,
         states: Query<&WidgetryListViewState>,
         mut changes: ResMut<CommittedChanges>| {
            let Ok(root_children) = roots.get(event.source) else {
                return;
            };
            let list = children.get(root_children[1]).unwrap()[0];
            assert_eq!(states.get(list).unwrap().selected, event.value);
            assert!(event.is_final);
            changes.0.push(event.value);
        },
    );
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A")).unwrap();
    let b = model.push(String::from("B")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    app.update();
    assert!(app.world().resource::<CommittedChanges>().0.is_empty());
    let field = app.world().get::<Children>(root).unwrap()[0];
    let popup = app.world().get::<Children>(root).unwrap()[1];
    app.world_mut().trigger(Activate { entity: field });
    for _ in 0..2 {
        WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, b);
    }
    app.world_mut().flush();
    assert_eq!(app.world().resource::<CommittedChanges>().0, vec![Some(b)]);
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    app.update();
    assert_eq!(rendered(app.world(), root).2, "1:B");
    let list = list(app.world(), root);
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    for _ in 0..2 {
        WidgetryComboBox::<String>::clear_selection(&mut app.world_mut().commands(), root);
    }
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<CommittedChanges>().0,
        vec![Some(b), None]
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .active,
        Some(b)
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(list));
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    app.update();
    assert!(
        app.world()
            .get::<Children>(content(app.world(), root))
            .is_none()
    );
    for id in [a, b, a] {
        WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, id);
    }
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<CommittedChanges>().0,
        vec![Some(b), None, Some(a), Some(b), Some(a)]
    );
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
}

#[test]
fn queued_programmatic_requests_validate_root_source_shell_and_state() {
    for failure in 0..5 {
        let mut app = app();
        let mut model = WidgetryListModel::default();
        let selected = model.push(String::from("A")).unwrap();
        let source = app.world_mut().spawn(model).id();
        let root = combo(&mut app, source);
        app.update();
        let list = list(app.world(), root);
        let state = *app.world().get::<WidgetryListViewState>(list).unwrap();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, app.world());
            WidgetryComboBox::<String>::set_selected(&mut commands, root, selected);
            WidgetryComboBox::<String>::clear_selection(&mut commands, root);
        }
        match failure {
            0 => {
                app.world_mut().despawn(root);
            }
            1 => {
                app.world_mut().despawn(source);
            }
            2 => {
                app.world_mut().despawn(list);
            }
            3 => {
                app.world_mut()
                    .entity_mut(list)
                    .remove::<WidgetryListViewState>();
            }
            _ => {
                let popup = app.world().get::<Children>(root).unwrap()[1];
                app.world_mut().entity_mut(popup).remove::<Visibility>();
            }
        }
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        errors.run(|| logs.run(|| queue.apply(app.world_mut())));
        let errors = errors.take();
        assert_eq!(errors.len(), 2);
        assert!(
            errors
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == bevy::log::Level::ERROR)
                .count(),
            2
        );
        assert!(app.world().resource::<Changes>().0.is_empty());
        if failure == 1 || failure == 4 {
            assert_eq!(app.world().get::<WidgetryListViewState>(list), Some(&state));
        } else {
            assert!(app.world().get::<WidgetryListViewState>(list).is_none());
        }
    }
}

#[test]
fn initial_selection_is_once_and_preserves_explicit_selection() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A")).unwrap();
    let b = model.push(String::from("B")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let automatic = combo(&mut app, source);
    let explicit = combo(&mut app, source);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), explicit, b);
    app.world_mut().flush();
    app.update();
    let automatic_list = list(app.world(), automatic);
    let explicit_list = list(app.world(), explicit);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(automatic_list)
            .unwrap()
            .selected,
        Some(a)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(explicit_list)
            .unwrap()
            .selected,
        Some(b)
    );
    WidgetryComboBox::<String>::clear_selection(&mut app.world_mut().commands(), automatic);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(automatic_list)
            .unwrap()
            .selected,
        None
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![Some(b), None]);
}

#[test]
fn field_cache_tracks_identity_index_and_revision() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A")).unwrap();
    model.push(String::from("B")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    app.update();
    let container = content(app.world(), root);
    let (old_wrapper, old_text, value) = rendered(app.world(), root);
    assert_eq!(value, "0:A");
    app.update();
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(1)
        .unwrap()
        .unwrap() = String::from("B2");
    app.update();
    assert_eq!(rendered(app.world(), root).0, old_wrapper);
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
        .unwrap()
        .unwrap() = String::from("A2");
    app.update();
    assert_eq!(rendered(app.world(), root).2, "0:A2");
    assert!(app.world().get_entity(old_wrapper).is_err());
    assert!(app.world().get_entity(old_text).is_err());
    let updated_wrapper = rendered(app.world(), root).0;
    assert!(
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .move_item(0, 1)
    );
    app.update();
    assert_eq!(rendered(app.world(), root).2, "1:A2");
    assert!(app.world().get_entity(updated_wrapper).is_err());
    assert_eq!(content(app.world(), root), container);
    let moved_wrapper = rendered(app.world(), root).0;
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(0, String::from("inserted"))
        .unwrap();
    app.update();
    assert_eq!(rendered(app.world(), root).2, "2:A2");
    assert!(app.world().get_entity(moved_wrapper).is_err());
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        Some(a)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

#[test]
fn same_selection_and_id_absent_from_source_preserve_projection() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let selected = model.push(String::from("selected")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    app.update();
    let original = rendered(app.world(), root).0;
    let mut other = WidgetryListModel::default();
    other.push(String::from("other first")).unwrap();
    let absent = other.push(String::from("other second")).unwrap();
    app.world_mut().spawn(other);
    app.set_error_handler(ErrorCapture::handler());
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, selected);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, absent);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    assert_eq!(errors.take().len(), 1);
    app.update();
    assert_eq!(rendered(app.world(), root).0, original);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        Some(selected)
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

fn assert_field_render_ready(app: &App, root: Entity) {
    let (_, text, _) = rendered(app.world(), root);
    let field = app.world().get::<Children>(root).unwrap()[0];
    assert!(
        app.world()
            .get::<bevy::ui::ComputedStackIndex>(text)
            .unwrap()
            .0
            > app
                .world()
                .get::<bevy::ui::ComputedStackIndex>(field)
                .unwrap()
                .0,
        "新 Field Text 当帧必须排在 Button background 之上"
    );
    assert!(
        app.world().get::<InheritedVisibility>(text).unwrap().get(),
        "改选当帧的新 Field Text 必须已经继承可见性"
    );
    assert!(
        app.world().get::<ComputedNode>(text).unwrap().size().x > 0.0,
        "改选当帧必须完成新 Field Text 的 layout"
    );
    assert!(
        !app.world()
            .get::<TextLayoutInfo>(text)
            .unwrap()
            .glyphs
            .is_empty(),
        "改选当帧必须有可绘制 glyph，不能等下一帧"
    );
}

#[test]
fn programmatic_selection_prepares_field_text_in_same_frame() {
    let mut app = app();
    add_ui_plugins(&mut app);
    // Bevy 消费阶段偶然排在构造之后会掩盖缺失的 schedule 依赖；在合法边界尽早消费，暴露新 Field 错过当帧准备的问题。
    app.configure_sets(
        PostUpdate,
        (
            VisibilitySystems::VisibilityPropagate,
            bevy::ui::UiSystems::Stack,
        )
            .before(bevy::ui::UiSystems::Propagate),
    );
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    advance_until(
        &mut app,
        Duration::from_secs(10),
        &format!("内建 Font {:?}", font.id()),
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .expect("内建字体应在期限内加载");
    app.set_default_font(bevy::text::FontSource::Handle(font));
    spawn_ui_camera(&mut app, UVec2::splat(400), 1.0);
    let mut model = WidgetryListModel::default();
    let first = model.push(String::from("Apple")).unwrap();
    let second = model.push(String::from("Orange")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    for _ in 0..3 {
        app.update();
    }
    let inserted = app
        .world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .insert(0, String::from("Inserted"))
        .unwrap();
    app.update();
    assert_field_render_ready(&app, root);
    for id in [inserted, second, first] {
        WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, id);
        app.world_mut().flush();
        app.update();
        assert_field_render_ready(&app, root);
    }
}

#[test]
fn empty_and_deleted_selection_preserve_field_shell() {
    let mut app = app();
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<String>::default())
        .id();
    let root = combo(&mut app, source);
    app.update();
    let container = content(app.world(), root);
    let field = app.world().get::<ChildOf>(container).unwrap().parent();
    let popup = app.world().get::<Children>(root).unwrap()[1];
    let icon = app.world().get::<Children>(field).unwrap()[1];
    let node = app.world().get::<Node>(field).unwrap().clone();
    let background = *app.world().get::<BackgroundColor>(field).unwrap();
    let border = *app.world().get::<BorderColor>(field).unwrap();
    assert!(app.world().get::<Children>(container).is_none());
    let a = app
        .world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push(String::from("A"))
        .unwrap();
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push(String::from("B"))
        .unwrap();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        None
    );
    assert!(app.world().get::<Children>(container).is_none());
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, a);
    app.world_mut().flush();
    app.update();
    let (old_wrapper, old_text, _) = rendered(app.world(), root);
    assert_eq!(
        app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .remove(0),
        Some(String::from("A"))
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        None
    );
    assert!(app.world().get::<Children>(container).is_none());
    assert!(app.world().get_entity(old_wrapper).is_err());
    assert!(app.world().get_entity(old_text).is_err());
    assert_eq!(content(app.world(), root), container);
    assert!(app.world().get::<Button>(field).is_some());
    assert!(app.world().get::<WidgetryIcon>(icon).is_some());
    assert_eq!(*app.world().get::<Node>(field).unwrap(), node);
    assert_eq!(
        *app.world().get::<BackgroundColor>(field).unwrap(),
        background
    );
    assert_eq!(*app.world().get::<BorderColor>(field).unwrap(), border);
    app.update();
    assert!(app.world().get::<Children>(container).is_none());
    assert_eq!(content(app.world(), root), container);
    app.world_mut().trigger(Activate { entity: field });
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![Some(a)]);
}

#[test]
fn field_reads_view_state_and_shared_model_updates_independent_views() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A")).unwrap();
    let b = model.push(String::from("B")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let first = combo(&mut app, source);
    let second = combo(&mut app, source);
    app.update();
    let second_list = list(app.world(), second);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), second, b);
    app.update();
    assert_eq!(rendered(app.world(), first).2, "0:A");
    assert_eq!(rendered(app.world(), second).2, "1:B");
    assert_eq!(app.world().resource::<Changes>().0, vec![Some(b)]);
    let second_wrapper = rendered(app.world(), second).0;
    app.world_mut().trigger(ValueChange {
        source: second_list,
        value: Some(a),
        is_final: true,
    });
    app.world_mut().flush();
    app.update();
    assert_eq!(app.world().resource::<Changes>().0, vec![Some(b), Some(a)]);
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(second_list)
            .unwrap()
            .selected,
        Some(b)
    );
    assert_eq!(rendered(app.world(), second).0, second_wrapper);
    assert_eq!(rendered(app.world(), second).2, "1:B");
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
        .unwrap()
        .unwrap() = String::from("A2");
    app.update();
    assert_eq!(rendered(app.world(), first).2, "0:A2");
    assert_eq!(rendered(app.world(), second).0, second_wrapper);
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    app.update();
    for root in [first, second] {
        assert_eq!(
            app.world()
                .get::<WidgetryListViewState>(list(app.world(), root))
                .unwrap()
                .selected,
            None
        );
        assert!(
            app.world()
                .get::<Children>(content(app.world(), root))
                .is_none()
        );
    }
}

#[test]
fn programmatic_selection_notifies_and_converges_to_last_valid_id() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A")).unwrap();
    let deleted = model.push(String::from("deleted")).unwrap();
    let b = model.push(String::from("B")).unwrap();
    model.remove(1);
    model.set_disabled(1, true);
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    let popup = app.world().get::<Children>(root).unwrap()[1];
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;
    let missing = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(missing);
    app.set_error_handler(ErrorCapture::handler());
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, b);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, a);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, b);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, deleted);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), source, a);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), missing, a);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), 3);
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        Some(b)
    );
    assert_eq!(rendered(app.world(), root).2, "1:B");
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![Some(b), Some(a), Some(b)]
    );
}

#[test]
fn clearing_model_after_partial_renderer_failure_clears_field() {
    let mut app = app();
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let source = app
        .world_mut()
        .spawn(WidgetryListModel::<String>::default())
        .id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox::<String> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, _: &String| bsn_list![
                    (Text("partial")),
                    (template(|_| Err::<Node, _>(BevyError::error("second child failed"))))
                ])},
            }
        })
        .unwrap()
        .id();
    let container = content(app.world(), root);
    app.update();
    let before = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    let selected = app
        .world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push(String::from("selected"))
        .unwrap();
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, selected);
    app.world_mut().flush();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    for _ in 0..2 {
        errors.run(|| logs.run(|| app.update()));
    }
    assert_eq!(errors.take().len(), 2);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.level == bevy::log::Level::ERROR)
            .count(),
        1
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .clear();
    errors.run(|| logs.run(|| app.update()));
    assert!(errors.take().is_empty());
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        None
    );
    assert!(
        app.world()
            .get::<Children>(container)
            .is_none_or(|children| children.is_empty()),
        "无 selection 的 Field 不能显示失败残留内容"
    );
    let after = app
        .world_mut()
        .query::<Entity>()
        .iter(app.world())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(
        after, before,
        "失败的 Field subtree 和预约 entity 必须全部清理"
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|r| r.fields.get("message").is_some_and(|m| m.contains("恢复")))
            .count(),
        1
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![Some(selected)]);
}
