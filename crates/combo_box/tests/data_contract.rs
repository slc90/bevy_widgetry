//! Coverage Map：本文件负责 generic/source/id 与构造诊断；selection_field.rs 负责 authority → Field 与同帧文本准备。
//! popup_composition.rs 负责真实输入、focus、关闭/恢复、动态 Text/Icon 与 Popup layout；bsn_combo_box.rs 负责 shell style/箭头。
//! State：有效/失效 source、未选择/已选择、root enabled/disabled 与共享 model 的独立 view。
//! Stimuli：BSN 配置、程序 selection、model move、source 销毁或 type 变化、disabled。
//! Guards：source/renderer 必填，尺寸合法，source 持有匹配 type 的 model。
//! Transitions：构造配置进入持久 Component；move 保留 stable selection；失效 source 报错；disable 拒绝用户输入。
//! Invariants：唯一 selection authority 位于内部 ListView，程序选择先提交再通知，source-local identity 不因 move 改变。
//! Couplings：共享 source 的多个 view 保持独立 selection，disabled 不修改 model。

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
#![cfg(test)]

use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ScrollArea, ValueChange};
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewItem,
    WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, primary_click, primary_press, scene_app};

#[test]
fn generic_scene_uses_independent_model() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<String>()
        .unwrap()
        .register_widgetry_combo_box::<String>()
        .unwrap()
        .register_widgetry_combo_box::<u32>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let selected = model.push(String::from("A")).unwrap();
    model.push(String::from("B")).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, item: &String| bsn_list![(Text({item.clone()}))])},
        }
    }).unwrap().id();
    app.update();
    let combo = app.world().get::<WidgetryComboBox<String>>(root).unwrap();
    assert_eq!(combo.source(), source);
    assert_eq!(combo.item_height(), 32.0);
    assert_eq!(combo.max_visible_items(), 8);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, selected);
    app.world_mut().flush();
    app.update();
    let list = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryListView<String>>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(selected)
    );
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .move_item(0, 1);
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(selected)
    );
}

struct Item(u32);

#[derive(Resource, Default)]
struct Changes(Vec<(Entity, Option<WidgetryListItemId>)>);

fn record(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    roots: Query<(), With<WidgetryComboBox<Item>>>,
    mut changes: ResMut<Changes>,
) {
    if roots.contains(event.source) {
        changes.0.push((event.source, event.value));
    }
}

fn combo(app: &mut App, source: Entity) -> Entity {
    app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<Item> {
            @source: source, @item_height: 24.0, @max_visible_items: 3,
            @renderer: {WidgetryListViewRenderer::new(|_, item: &Item| bsn_list![(Text({item.0.to_string()}))])},
        }
    }).unwrap().id()
}

fn list(world: &World, root: Entity) -> Entity {
    let popup = world.get::<Children>(root).unwrap()[1];
    world.get::<Children>(popup).unwrap()[0]
}

#[test]
fn shared_source_and_root_notifications() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<Item>()
        .unwrap()
        .init_resource::<Changes>()
        .add_observer(record);
    let mut model = WidgetryListModel::default();
    let a = model.push(Item(1)).unwrap();
    let b = model.push(Item(2)).unwrap();
    let source = app.world_mut().spawn(model).id();
    let first = combo(&mut app, source);
    let second = combo(&mut app, source);
    app.world_mut()
        .entity_mut(first)
        .insert(InteractionDisabled);
    WidgetryComboBox::<Item>::set_selected(&mut app.world_mut().commands(), first, a);
    app.world_mut().flush();
    app.update();
    let first_list = list(app.world(), first);
    let second_list = list(app.world(), second);
    assert!(app.world().get::<InteractionDisabled>(first_list).is_some());
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(first_list)
            .unwrap()
            .selected,
        Some(a)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(second_list)
            .unwrap()
            .selected,
        Some(a)
    );
    assert_eq!(app.world().resource::<Changes>().0, vec![(first, Some(a))]);
    WidgetryListView::<Item>::set_selected(&mut app.world_mut().commands(), second_list, 1);
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<Changes>().0,
        vec![(first, Some(a)), (second, Some(b))]
    );
    let config = app.world().get::<WidgetryComboBox<Item>>(second).unwrap();
    assert_eq!(config.item_height(), 24.0);
    assert_eq!(config.max_visible_items(), 3);
    let scene = config.renderer().render(0, &Item(42)).unwrap();
    app.world_mut()
        .spawn_scene(bsn! { Node Children [{scene}] })
        .unwrap();
}

#[test]
fn queued_selection_does_not_drift_after_model_move() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<Item>().unwrap();
    let mut model = WidgetryListModel::default();
    let selected = model.push(Item(1)).unwrap();
    model.push(Item(2)).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    WidgetryComboBox::<Item>::set_selected(&mut app.world_mut().commands(), root, selected);
    app.world_mut().commands().queue(move |world: &mut World| {
        world
            .get_mut::<WidgetryListModel<Item>>(source)
            .unwrap()
            .move_item(0, 1);
    });
    app.world_mut().flush();
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list(app.world(), root))
            .unwrap()
            .selected,
        Some(selected)
    );
}

#[test]
fn invalid_construction_returns_error_and_logs() {
    for (source_missing, renderer_missing, height, count) in [
        (true, false, 32.0, 8usize),
        (false, true, 32.0, 8),
        (false, false, 0.0, 8),
        (false, false, f32::NAN, 8),
        (false, false, 32.0, 0),
        (false, false, f32::MAX, 8),
    ] {
        let capture = LogCapture::default();
        let mut app = scene_app();
        app.register_widgetry_combo_box::<Item>().unwrap();
        let source = if source_missing {
            Entity::PLACEHOLDER
        } else {
            app.world_mut()
                .spawn(WidgetryListModel::<Item>::default())
                .id()
        };
        let renderer = if renderer_missing {
            WidgetryListViewRenderer::default()
        } else {
            WidgetryListViewRenderer::new(|_, _: &Item| bsn_list![])
        };
        assert!(capture.run(|| {
            app.world_mut().spawn_scene(bsn! {
                @WidgetryComboBox::<Item> { @source: source, @renderer: {renderer}, @item_height: height, @max_visible_items: count }
            })
        }).is_err());
        assert!(
            capture
                .records()
                .iter()
                .any(|record| record.level == bevy::log::Level::ERROR)
        );
    }
}

#[test]
fn invalid_source_uses_listview_invariant_diagnostic() {
    for kind in 0..3 {
        let capture = LogCapture::default();
        let mut app = scene_app();
        app.register_widgetry_combo_box::<Item>().unwrap();
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let source = match kind {
            0 => {
                let entity = app.world_mut().spawn_empty().id();
                app.world_mut().despawn(entity);
                entity
            }
            1 => app.world_mut().spawn_empty().id(),
            _ => app
                .world_mut()
                .spawn(WidgetryListModel::<String>::default())
                .id(),
        };
        combo(&mut app, source);
        app.set_error_handler(ErrorCapture::handler());
        let errors = ErrorCapture::default();
        errors.run(|| capture.run(|| app.world_mut().run_schedule(PreUpdate)));
        let errors = errors.take();
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
        assert!(capture.records().iter().any(|record| {
            record.level == bevy::log::Level::ERROR
                && record
                    .fields
                    .get("message")
                    .is_some_and(|message| message.contains("ListView source"))
        }));
    }
}

#[test]
fn disabling_before_pointer_input_blocks_selection_and_focus() {
    let mut app = scene_app();
    app.init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>();
    app.world_mut().register_component::<Window>();
    app.register_widgetry_combo_box::<Item>().unwrap();
    let mut model = WidgetryListModel::default();
    model.push(Item(1)).unwrap();
    let selected = model.push(Item(2)).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    let list = list(app.world(), root);
    let field = app.world().get::<Children>(root).unwrap()[0];
    let viewport = app
        .world()
        .get::<Children>(list)
        .unwrap()
        .iter()
        .find(|&child| app.world().get::<ScrollArea>(child).is_some())
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(200.0, 72.0),
        content_size: Vec2::new(200.0, 72.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    app.update();
    let initial_state = *app.world().get::<WidgetryListViewState>(list).unwrap();
    let row = app
        .world_mut()
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(app.world())
        .find_map(|(entity, item)| (item.id == selected).then_some(entity))
        .unwrap();
    app.world_mut().trigger(Activate { entity: field });
    let initial_focus = app.world().resource::<InputFocus>().get();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.world_mut().trigger(primary_press(row));
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(list).unwrap(),
        initial_state
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), initial_focus);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.world_mut().trigger(primary_click(row));
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        Some(selected)
    );
}
