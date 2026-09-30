#![cfg(test)]

use bevy::camera::visibility::VisibilitySystems;
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, Button, ValueChange};
use bevy_widgetry_asset::BuiltinFont;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_test_utils::{add_ui_plugins, advance_until, scene_app, spawn_ui_camera};
use std::time::Duration;

/// 只收集公共 root 通知，用于区分真实用户交互与静默 projection。
#[derive(Resource, Default)]
struct Changes(Vec<WidgetryListItemId>);

/// 捕获 ComboBox root 发出的用户通知。
fn record(
    event: On<ValueChange<WidgetryListItemId>>,
    roots: Query<(), With<WidgetryComboBox<String>>>,
    mut changes: ResMut<Changes>,
) {
    if roots.contains(event.source) {
        changes.0.push(event.value);
    }
}

/// 提供真实 Scene runtime，renderer 同时依赖 index 与业务内容。
fn app() -> App {
    let mut app = scene_app();
    app.register_widgetry_combo_box::<String>()
        .init_resource::<Changes>()
        .add_observer(record);
    app
}

/// 使用公开 BSN contract，renderer 生成嵌套内容以验证递归清理。
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

/// 按组合 hierarchy 读取真正的 ListView state，不依赖私有 marker。
fn list(world: &World, root: Entity) -> Entity {
    let popup = world.get::<Children>(root).unwrap()[1];
    world.get::<Children>(popup).unwrap()[0]
}

/// Field content container 始终为 Button 的首个 child。
fn content(world: &World, root: Entity) -> Entity {
    let field = world.get::<Children>(root).unwrap()[0];
    world.get::<Children>(field).unwrap()[0]
}

/// 读取 renderer 创建的嵌套 Text，同时返回 subtree identity 用于检测 rebuild。
fn rendered(world: &World, root: Entity) -> (Entity, Entity, &str) {
    let wrapper = world.get::<Children>(content(world, root)).unwrap()[0];
    let text = world.get::<Children>(wrapper).unwrap()[0];
    (wrapper, text, &world.get::<Text>(text).unwrap().0)
}

/// 非空 model 默认第一项，只初始化一次；初次 Update 前的显式 selection 优先。
#[test]
fn initial_selection_is_once_and_preserves_explicit_selection() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A"));
    let b = model.push(String::from("B"));
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
    app.world_mut()
        .get_mut::<WidgetryListViewState>(automatic_list)
        .unwrap()
        .selected = None;
    app.update();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(automatic_list)
            .unwrap()
            .selected,
        None
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

/// 初始 Field 从真实 selection render；稳定帧与非 selected revision 不重建，selected revision/move 重建。
#[test]
fn field_cache_tracks_identity_index_and_revision() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A"));
    model.push(String::from("B"));
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
        .unwrap() = String::from("B2");
    app.update();
    assert_eq!(rendered(app.world(), root).0, old_wrapper);
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
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

/// 同值程序化选择保持 renderer subtree；其他 model 分配但本 source 不存在的 id 为 no-op。
#[test]
fn same_selection_and_id_absent_from_source_preserve_projection() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let selected = model.push(String::from("selected"));
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    app.update();
    let original = rendered(app.world(), root).0;
    let mut other = WidgetryListModel::default();
    other.push(String::from("other first"));
    let absent = other.push(String::from("other second"));
    app.world_mut().spawn(other);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, selected);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, absent);
    app.world_mut().flush();
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

/// 验证新 Field Text 的真实绘制前置条件，避免只检查文本值而漏掉一帧空白。
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

/// 在真实 visibility/stack/文本 layout 中执行 insert first → set selected first，新 Field 当帧即可绘制。
#[test]
fn programmatic_selection_prepares_field_text_in_same_frame() {
    let mut app = app();
    add_ui_plugins(&mut app);
    // 在共享契约允许的范围内尽早执行消费阶段，验证 Field 不依赖偶然的 system 顺序。
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
    let first = model.push(String::from("Apple"));
    let second = model.push(String::from("Orange"));
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

/// 首次为空后 push 不自动选择；删除 selected item 清空 subtree，Button、icon 与 Activate 仍完整。
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
        .push(String::from("A"));
    app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .push(String::from("B"));
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
    app.world_mut().trigger(Activate { entity: field });
    assert_eq!(
        *app.world().get::<Visibility>(popup).unwrap(),
        Visibility::Visible
    );
    assert!(app.world().resource::<Changes>().0.is_empty());
}

/// 直接改 authority 更新 Field，孤立 ValueChange 不驱动 Field；共享 model 的两个 view 保持独立。
#[test]
fn field_reads_view_state_and_shared_model_updates_independent_views() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A"));
    let b = model.push(String::from("B"));
    let source = app.world_mut().spawn(model).id();
    let first = combo(&mut app, source);
    let second = combo(&mut app, source);
    app.update();
    let second_list = list(app.world(), second);
    app.world_mut()
        .get_mut::<WidgetryListViewState>(second_list)
        .unwrap()
        .selected = Some(b);
    app.update();
    assert_eq!(rendered(app.world(), first).2, "0:A");
    assert_eq!(rendered(app.world(), second).2, "1:B");
    assert!(app.world().resource::<Changes>().0.is_empty());
    let second_wrapper = rendered(app.world(), second).0;
    app.world_mut().trigger(ValueChange {
        source: second_list,
        value: a,
        is_final: true,
    });
    app.world_mut().flush();
    app.update();
    assert_eq!(rendered(app.world(), second).0, second_wrapper);
    assert_eq!(rendered(app.world(), second).2, "1:B");
    *app.world_mut()
        .get_mut::<WidgetryListModel<String>>(source)
        .unwrap()
        .get_mut(0)
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

/// root/item disabled 允许程序化改选；无效 root/id 为 no-op，连续排队收敛到最后有效选择且不关闭 Popup。
#[test]
fn programmatic_selection_is_silent_and_converges_to_last_valid_id() {
    let mut app = app();
    let mut model = WidgetryListModel::default();
    let a = model.push(String::from("A"));
    let deleted = model.push(String::from("deleted"));
    let b = model.push(String::from("B"));
    model.remove(1);
    model.set_disabled(1, true);
    let source = app.world_mut().spawn(model).id();
    let root = combo(&mut app, source);
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    let popup = app.world().get::<Children>(root).unwrap()[1];
    *app.world_mut().get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, b);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, a);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, b);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), root, deleted);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), source, a);
    let missing = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(missing);
    WidgetryComboBox::<String>::set_selected(&mut app.world_mut().commands(), missing, a);
    app.world_mut().flush();
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
    assert!(app.world().resource::<Changes>().0.is_empty());
}
