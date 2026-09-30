//! Coverage Map：本文件负责 generic/source/id 与构造诊断；selection_field.rs 负责 authority→Field 与同帧文本准备。
//! popup_composition.rs 负责真实 Button/ListView 输入、focus、关闭/恢复、动态 Text/Icon 与 popup layout；bsn_combo_box.rs 保留 shell 样式/箭头构造。
//! 跨域 invariant：唯一 selection authority 位于内部 ListView；程序选择静默，source-local identity 不受 move 影响。

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
use bevy_widgetry_test_utils::{LogCapture, primary_click, primary_press, scene_app};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// 公共 BSN 配置由持久 component 承接，移动 item 后保持 stable selection。
#[test]
fn generic_scene_uses_independent_model() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<String>()
        .register_widgetry_combo_box::<String>()
        .register_widgetry_combo_box::<u32>();
    let mut model = WidgetryListModel::default();
    let selected = model.push(String::from("A"));
    model.push(String::from("B"));
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

/// 刻意不实现 Clone / Default，保护泛型边界。
struct Item(u32);

/// 收集 ComboBox root 用户通知，不把内部 ListView event 当作公共输出。
#[derive(Resource, Default)]
struct Changes(Vec<(Entity, WidgetryListItemId)>);

/// 只在 ComboBox root 收集事件。
fn record(
    event: On<ValueChange<WidgetryListItemId>>,
    roots: Query<(), With<WidgetryComboBox<Item>>>,
    mut changes: ResMut<Changes>,
) {
    if roots.contains(event.source) {
        changes.0.push((event.source, event.value));
    }
}

/// 使用非 Clone type 和指定配置构造真实组合。
fn combo(app: &mut App, source: Entity) -> Entity {
    app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<Item> {
            @source: source, @item_height: 24.0, @max_visible_items: 3,
            @renderer: {WidgetryListViewRenderer::new(|_, item: &Item| bsn_list![(Text({item.0.to_string()}))])},
        }
    }).unwrap().id()
}

/// 从 ComboBox 的 hierarchy 找到内部 ListView，验证公开 composition 边界。
fn list(world: &World, root: Entity) -> Entity {
    let popup = world.get::<Children>(root).unwrap()[1];
    world.get::<Children>(popup).unwrap()[0]
}

/// 同一 source 可被多个 ComboBox 共用，程序化设置静默且各自 selection 独立。
#[test]
fn shared_source_and_root_notifications() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<Item>()
        .init_resource::<Changes>()
        .add_observer(record);
    let mut model = WidgetryListModel::default();
    let a = model.push(Item(1));
    let b = model.push(Item(2));
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
    assert!(app.world().resource::<Changes>().0.is_empty());
    app.world_mut()
        .get_mut::<WidgetryListViewState>(second_list)
        .unwrap()
        .selected = Some(b);
    app.world_mut().trigger(ValueChange {
        source: second_list,
        value: b,
        is_final: true,
    });
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Changes>().0, vec![(second, b)]);
    let config = app.world().get::<WidgetryComboBox<Item>>(second).unwrap();
    assert_eq!(config.item_height(), 24.0);
    assert_eq!(config.max_visible_items(), 3);
    let scene = config.renderer().render(0, &Item(42));
    app.world_mut()
        .spawn_scene(bsn! { Node Children [{scene}] })
        .unwrap();
}

/// 排队选择与 model 移动交错时，不能把早先计算的 index 当作最终 identity。
#[test]
fn queued_selection_does_not_drift_after_model_move() {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<Item>();
    let mut model = WidgetryListModel::default();
    let selected = model.push(Item(1));
    model.push(Item(2));
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

/// 缺少 source / renderer、非法尺寸与零行数均必须在构造时记录 ERROR 后拒绝。
#[test]
fn invalid_construction_logs_before_panicking() {
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
        app.register_widgetry_combo_box::<Item>();
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
        assert!(capture.run(|| catch_unwind(AssertUnwindSafe(|| {
            app.world_mut().spawn_scene(bsn! {
                @WidgetryComboBox::<Item> { @source: source, @renderer: {renderer}, @item_height: height, @max_visible_items: count }
            }).unwrap();
        }))).is_err());
        assert!(
            capture
                .records()
                .iter()
                .any(|record| record.level == bevy::log::Level::ERROR)
        );
    }
}

/// source 被销毁、缺少 model 或持有错误业务 type 时，真实内部 ListView 在 Update 记录 ERROR 并拒绝运行。
#[test]
fn invalid_source_uses_listview_invariant_diagnostic() {
    for kind in 0..3 {
        let capture = LogCapture::default();
        let mut app = scene_app();
        app.register_widgetry_combo_box::<Item>();
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
        assert!(
            capture
                .run(|| catch_unwind(AssertUnwindSafe(|| app.update())))
                .is_err()
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

/// root 在输入前禁用时，不等待下一次 PreUpdate；内部 selection / active / focus 均保持不变。
#[test]
fn disabling_before_pointer_input_blocks_selection_and_focus() {
    let mut app = scene_app();
    app.init_resource::<UiScale>()
        .init_resource::<ButtonInput<KeyCode>>();
    app.world_mut().register_component::<Window>();
    app.register_widgetry_combo_box::<Item>();
    let mut model = WidgetryListModel::default();
    model.push(Item(1));
    let selected = model.push(Item(2));
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
