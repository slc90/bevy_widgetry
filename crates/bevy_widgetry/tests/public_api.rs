//! Coverage Map：本文件保留编译/Scene 构造 smoke，另验证公开 state 消费、字体策略、插件与有效 MessageBox runtime。
//! focus.rs 负责跨 Widget 的真实 pointer/keyboard focus 归属和隐藏 Popup 输入隔离。
//! 构造 smoke 不声称 asset/OS 有效；runtime 使用合法 source/parent，Widget 入口全部来自 facade。

#![cfg(test)]

use bevy::text::FontSource;
use bevy::{app::App, color::Color, ecs::entity::Entity, prelude::*};
use bevy_widgetry::button::{WidgetryButton, WidgetryButtonPlugin};
use bevy_widgetry::check_box::{
    WidgetryCheckBox, WidgetryCheckBoxPlugin, WidgetryCheckState, WidgetryTriStateCheckbox,
};
use bevy_widgetry::combo_box::{
    WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxPlugin, WidgetryComboBoxProps,
};
use bevy_widgetry::icon::{WidgetryIcon, WidgetryIconPlugin, WidgetryIconProps};
use bevy_widgetry::list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewAppExt, WidgetryListViewItem,
    WidgetryListViewPlugin, WidgetryListViewProps, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry::message_box::{
    WidgetryMessageBox, WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin,
    WidgetryMessageBoxResult, WidgetryMessageBoxResultEvent, widgetry_message_box,
};
use bevy_widgetry::radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupPlugin, WidgetryRadioOption,
};
use bevy_widgetry::scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
    WidgetryScrollAreaContent, WidgetryScrollAreaPlugin, WidgetryScrollAreaProps,
    WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
use bevy_widgetry::style::WidgetryAppExt;
use bevy_widgetry::style::{
    ColorTheme, DARK_THEME, ForegroundColor, LIGHT_THEME, ThemeChanged, ThemeMode, ThemePlugin,
};
use bevy_widgetry::text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use bevy_widgetry::tooltip::{
    TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin, WidgetryTooltipProps,
};
use bevy_widgetry::tree::{
    WidgetryTreeAppExt, WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeRenderer,
    WidgetryTreeView, WidgetryTreeVisibleItem,
};
use bevy_widgetry::window::{
    WidgetryWindowControlsConfig, WidgetryWindowPlugin, owned_widgetry_window, widgetry_window,
};
use bevy_widgetry_test_utils::{press, primary_click, scene_app};

/// facade 消费者观察公开 result source/value，内部 action 与 marker 不参与断言。
#[derive(Resource, Default)]
struct DialogResults(Vec<(Entity, WidgetryMessageBoxResult)>);

/// 无 Default/Clone 的业务 type，用于避免 API 无意增加额外 generic bound。
struct FileEntry {
    /// renderer 显示的业务内容。
    name: String,
}

/// 消费者的业务 Component 无 Clone/Default bound，renderer 注册入口来自 facade。
#[derive(Component)]
struct TreeEntry(String);

/// facade 能独立注册 Tree renderer、构造有真实 hierarchy 的 TreeView 并观察 Entity selection projection。
#[test]
fn tree_consumer_can_register_renderers_and_use_entity_selection_through_facade() {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<UiScale>();
    app.register_renderer::<TreeEntry>(WidgetryTreeRenderer::new(|_, entry: &TreeEntry| {
        bsn_list![(Text({ entry.0.clone() }))]
    }));
    let root = app.world_mut().spawn_empty().id();
    let node = app
        .world_mut()
        .spawn((
            WidgetryTreeNode,
            TreeEntry("facade Tree".into()),
            ChildOf(root),
        ))
        .id();
    let source = app.world_mut().spawn(WidgetryTreeModel::new(root)).id();
    let view = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
        .unwrap()
        .id();
    let list = app.world().get::<Children>(view).unwrap()[0];
    let viewport = app
        .world()
        .get::<Children>(list)
        .unwrap()
        .iter()
        .find(|&child| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(child)
                .is_some()
        })
        .unwrap();
    app.world_mut().entity_mut(viewport).insert(ComputedNode {
        size: Vec2::new(200.0, 64.0),
        inverse_scale_factor: 1.0,
        ..default()
    });
    assert!(WidgetryTreeModel::select(
        app.world_mut(),
        source,
        Some(node)
    ));
    app.update();
    assert_eq!(
        app.world().get::<WidgetryTreeView>(view).unwrap().source(),
        source
    );
    let id = app
        .world()
        .get::<WidgetryListViewState>(list)
        .unwrap()
        .selected
        .unwrap();
    assert_eq!(
        app.world()
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .unwrap()
            .get_by_id(id)
            .unwrap()
            .entity,
        node
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "facade Tree")
    );
}

/// 消费者只通过 facade 注册多个 T，并用同一个 model 构造相互独立的 view state。
#[test]
fn generic_api_is_available_through_facade() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<FileEntry>()
        .register_widgetry_list_view::<FileEntry>()
        .register_widgetry_list_view::<u32>();
    let mut model = WidgetryListModel::default();
    let id = model.push(FileEntry {
        name: "report".into(),
    });
    let source = app.world_mut().spawn(model).id();
    let renderer = WidgetryListViewRenderer::new(|index, entry: &FileEntry| {
        bsn_list![(Text(format!("{index}: {}", entry.name)))]
    });
    let first = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryListView::<FileEntry> {
                @source: source,
                @renderer: {renderer.clone()},
            }
        })
        .unwrap()
        .id();
    let second = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryListView::<FileEntry> {
                @source: source,
                @item_height: 48.0,
                @renderer: renderer,
            }
        })
        .unwrap()
        .id();
    let other_source = app
        .world_mut()
        .spawn(WidgetryListModel::<u32>::default())
        .id();
    let other = app.world_mut().spawn_scene(bsn! {
        @WidgetryListView::<u32> {
            @source: other_source,
            @renderer: {WidgetryListViewRenderer::new(|_, value: &u32| bsn_list![(Text({value.to_string()}))])},
        }
    }).unwrap().id();
    app.update();
    let view = app
        .world()
        .get::<WidgetryListView<FileEntry>>(first)
        .unwrap();
    assert_eq!(view.source(), source);
    assert_eq!(view.item_height(), 32.0);
    assert_eq!(
        app.world()
            .get::<WidgetryListView<FileEntry>>(second)
            .unwrap()
            .item_height(),
        48.0
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListView<u32>>(other)
            .unwrap()
            .source(),
        other_source
    );
    WidgetryListView::<FileEntry>::set_selected(&mut app.world_mut().commands(), first, 0);
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(first)
            .unwrap()
            .active,
        Some(id)
    );
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(second)
            .unwrap()
            .selected,
        None
    );
    let marker = WidgetryListViewItem { id, index: 0 };
    assert_eq!(marker.id, id);
    let props = WidgetryListViewProps::<FileEntry>::default();
    assert_eq!(props.item_height, 32.0);
    for root in [first, second, other] {
        assert!(app.world().get::<WidgetryScrollArea>(root).is_some());
        let viewport = app
            .world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|&child| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(child)
                    .is_some()
            })
            .unwrap();
        assert!(
            app.world()
                .get::<bevy::ui::ScrollPosition>(viewport)
                .is_some()
        );
        let content = app.world().get::<Children>(viewport).unwrap()[0];
        assert!(
            app.world()
                .get::<WidgetryScrollAreaContent>(content)
                .is_some()
        );
        assert_eq!(app.world().get::<Children>(content).unwrap().len(), 2);
    }
}

/// 消费者只通过 facade 构造空与自定义内容 ScrollArea，并用公开 Viewport 访问原生 ScrollPosition。
#[test]
fn scroll_area_scene_api_is_usable() {
    let props = WidgetryScrollAreaProps::default();
    assert_eq!(props.axis, ScrollAxis::Vertical);
    assert_eq!(props.scrollbar_visibility.horizontal, ScrollbarPolicy::Auto);
    assert_eq!(props.scrollbar_visibility.vertical, ScrollbarPolicy::Auto);
    assert_eq!(props.scrollbar_thickness, 12.0);
    assert!(props.keyboard_scroll);
    assert!(props.content.is_none() && props.children.is_none());

    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryScrollAreaPlugin);
    let empty = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryScrollArea })
        .unwrap()
        .id();
    let configured = app.world_mut().spawn_scene(bsn! {
        @WidgetryScrollArea {
            @axis: ScrollAxis::Both,
            @scrollbar_visibility: {ScrollbarVisibility { horizontal: ScrollbarPolicy::Always, vertical: ScrollbarPolicy::Auto }},
            @content: bsn! { Node { padding: UiRect::all(px(4)) } },
            @children: bsn_list![(Node { width: px(120), height: px(160) })],
        }
    }).unwrap().id();
    assert!(app.world().get::<WidgetryScrollArea>(empty).is_some());
    assert!(app.world().get::<WidgetryScrollArea>(configured).is_some());
    let _ = WidgetryScrollIntoView { entity: configured };

    for root in [empty, configured] {
        let viewport = app
            .world()
            .get::<Children>(root)
            .unwrap()
            .iter()
            .find(|&child| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(child)
                    .is_some()
            })
            .unwrap();
        assert!(
            app.world()
                .get::<bevy::ui::ScrollPosition>(viewport)
                .is_some()
        );
        let content = app.world().get::<Children>(viewport).unwrap()[0];
        assert_eq!(
            app.world().get::<Children>(content).unwrap().len(),
            usize::from(root == configured)
        );
    }
}

// facade 暴露完整 RadioGroup BSN 与静默选择 API，消费者无需直接引用功能 crate。
#[test]
fn radio_group_scene_api_is_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryRadioGroup
        Children [(@WidgetryRadioOption Children [Text("Low")]), (@WidgetryRadioOption Children [Text("High")])]
    }).unwrap().id();
    WidgetryRadioGroup::set_selected(&mut app.world_mut().commands(), root, 1);
    app.world_mut().flush();
    app.update();
    let children = app.world().get::<Children>(root).unwrap();
    assert!(app.world().get::<WidgetryRadioGroup>(root).is_some());
    assert!(
        app.world()
            .get::<WidgetryRadioOption>(children[1])
            .is_some()
    );
    assert!(app.world().get::<bevy::ui::Checked>(children[1]).is_some());
    assert!(app.world().get::<bevy::ui::Checked>(children[0]).is_none());
}

// Window plugin 继续为普通 Bevy 文本自动安装内建 fallback。
#[test]
fn window_plugin_installs_app_font_fallback() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<ButtonInput<MouseButton>>();
    app.add_plugins(WidgetryWindowPlugin);
    let entity = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert!(matches!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::Handle(_)
    ));
}

// facade 的 TextField 可通过 BSN 构造；预装官方 TabNavigationPlugin 不应重复注册或改变字体策略。
#[test]
fn text_field_scene_preserves_app_font_policy() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        bevy::input_focus::InputFocusPlugin,
    ));
    app.add_plugins((
        bevy::input_focus::tab_navigation::TabNavigationPlugin,
        WidgetryTextFieldPlugin,
    ));
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTextField })
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<WidgetryTextField>(entity).is_some());
    assert!(
        app.world()
            .get::<bevy::text::EditableText>(entity)
            .is_some()
    );
    assert_eq!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::default()
    );
}

// facade 的 ReadOnly TextField 通过 BSN 构造后保留官方 EditableText。
#[test]
fn read_only_text_field_scene_api_is_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
        .unwrap()
        .id();
    app.update();
    assert!(
        app.world()
            .get::<WidgetryReadOnlyTextField>(entity)
            .is_some()
    );
    assert!(app.world().get::<WidgetryTextField>(entity).is_none());
    assert!(
        app.world()
            .get::<bevy::text::EditableText>(entity)
            .is_some()
    );
}

// Button 不创建文本，独立注册时不需要 asset 设施，也不应改写调用方文本的默认字体。
#[test]
fn button_plugin_leaves_default_font_unchanged() {
    let mut app = App::new();
    app.add_plugins(WidgetryButtonPlugin);
    let entity = app.world_mut().spawn(TextFont::default()).id();
    app.update();
    assert_eq!(
        app.world().get::<TextFont>(entity).unwrap().font,
        FontSource::default()
    );
}

// 从 facade 导入消费者需要的 type，验证重构后公开入口仍可构造。
#[test]
fn facade_public_types_are_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.register_widgetry_combo_box::<String>();
    let _ = WidgetryComboBoxPlugin;
    let props = WidgetryComboBoxProps::<String>::default();
    assert_eq!(props.source, Entity::PLACEHOLDER);
    assert_eq!(props.item_height, 32.0);
    assert_eq!(props.max_visible_items, 8);
    assert!(std::mem::size_of::<WidgetryComboBox<String>>() > 0);
    let _ = ForegroundColor(Color::WHITE);
    let _ = bevy_widgetry::style::z_index::TOOLTIP;
}

// facade 的 Tooltip module 提供完整 styled API，消费者可用任意 SceneList factory 构造 anchor。
#[test]
fn tooltip_scene_api_is_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.init_resource::<bevy::picking::hover::HoverMap>()
        .add_plugins(WidgetryTooltipPlugin);
    let _ = WidgetryTooltipProps::default();
    let anchor = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list![Node, Text("Details")])} }
        })
        .unwrap()
        .id();
    assert!(app.world().get::<WidgetryTooltip>(anchor).is_some());
}

// 同时装配多个 style plugin，验证共享 theme 设施不会重复注册且可使用外部 theme。
#[test]
fn style_theme_api_and_plugins_work_together() {
    let _: &ColorTheme = &DARK_THEME;
    assert_eq!(ThemeMode::Light.colors(), &LIGHT_THEME);
    let mut app = App::new();
    app.set_default_font(FontSource::Monospace);
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    app.insert_resource(ThemeMode::Light).add_plugins((
        ThemePlugin,
        WidgetryButtonPlugin,
        WidgetryComboBoxPlugin,
    ));
    app.world_mut().trigger(ThemeChanged {
        mode: ThemeMode::Light,
    });
    app.update();
    assert_eq!(*app.world().resource::<ThemeMode>(), ThemeMode::Light);
}

// 消费者仅通过 facade 与 BSN 创建 icon，无需取得 AssetServer，运行期 component 仍可用于 query。
#[test]
fn icon_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryIconPlugin,
    ));
    let _ = WidgetryIconProps::default();
    let plain = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! { @WidgetryIcon { @path: "some/icon.svg" } })
        .id();
    let configured = app
        .world_mut()
        .commands()
        .spawn_scene(bsn! {
            @WidgetryIcon {
                @path: "some/icon.svg",
                @max_size: { Some(UVec2::new(24, 24)) },
                @color: { Some(Color::WHITE) },
            }
        })
        .id();
    app.world_mut().flush();
    for entity in [plain, configured] {
        assert!(app.world().get::<WidgetryIcon>(entity).is_some());
        assert!(app.world().get::<Node>(entity).is_some());
    }
}

// 从 facade 组合空 title bar 与主体 Scene，验证新的 Window 公开入口可直接用于 BSN。
#[test]
fn window_scene_api_is_usable() {
    let _ = WidgetryWindowPlugin;
    let config = WidgetryWindowControlsConfig::default();
    assert!(config.minimize_visible && config.maximize_visible);
    let _ = bsn! {
        widgetry_window(Entity::PLACEHOLDER, Entity::PLACEHOLDER, config, bsn_list![], bsn_list![(Text("Body"))])
    };
}

/// facade 提供 WidgetryMessageBox type 与 BSN function，消费者不需要直接依赖内部 crate。
#[test]
fn message_box_scene_api_is_usable() {
    let _ = WidgetryMessageBox;
    let _ = WidgetryMessageBoxPlugin;
    let _ = WidgetryMessageBoxResultEvent {
        entity: Entity::PLACEHOLDER,
        result: WidgetryMessageBoxResult::Ok,
    };
    let _ = bsn! {
        widgetry_message_box(Entity::PLACEHOLDER, "Confirm", WidgetryMessageBoxButtons::YesNoCancel, bsn_list![(Text("Save changes?"))])
    };
}

// 消费者仅通过 facade 创建完整 Button Scene，保留可 query 的身份与官方行为 component。
#[test]
fn button_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.set_default_font(FontSource::Monospace);
    app.add_plugins(WidgetryButtonPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryButton })
        .unwrap()
        .id();
    app.update();
    assert!(app.world().get::<WidgetryButton>(entity).is_some());
    assert!(
        app.world()
            .get::<bevy::ui_widgets::Button>(entity)
            .is_some()
    );
    assert_eq!(
        app.world().get::<BackgroundColor>(entity).unwrap().0,
        DARK_THEME.control_background
    );
}

/// 消费者仅通过 facade 构造两类 CheckBox，并调用三态静默程序化 API。
#[test]
fn check_box_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.init_asset::<Image>()
        .add_plugins(WidgetryCheckBoxPlugin);
    let binary = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryCheckBox })
        .unwrap()
        .id();
    let tri = app
        .world_mut()
        .spawn_scene(bsn! { @WidgetryTriStateCheckbox })
        .unwrap()
        .id();
    WidgetryTriStateCheckbox::set_state(
        &mut app.world_mut().commands(),
        tri,
        WidgetryCheckState::Checked,
    );
    WidgetryTriStateCheckbox::cycle_state(&mut app.world_mut().commands(), tri);
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<WidgetryCheckBox>(binary).is_some());
    assert_eq!(
        app.world().get::<WidgetryCheckState>(tri),
        Some(&WidgetryCheckState::Indeterminate)
    );
}

// facade 提供完整 ComboBox BSN 入口与静默 selection API，plugin 不隐式改变调用方字体策略。
#[test]
fn combo_box_scene_api_is_usable_without_installing_font_fallback() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        bevy::input_focus::InputFocusPlugin,
    ));
    app.init_asset::<Image>()
        .add_plugins(WidgetryComboBoxPlugin)
        .register_widgetry_combo_box::<u32>();
    let font = app.world_mut().spawn(TextFont::default()).id();
    let mut model = WidgetryListModel::default();
    model.push(0u32);
    let selected = model.push(1u32);
    let source = app.world_mut().spawn(model).id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox::<u32> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, _: &u32| bsn_list![Node])},
            }
        })
        .unwrap()
        .id();
    WidgetryComboBox::<u32>::set_selected(&mut app.world_mut().commands(), root, selected);
    app.world_mut().flush();
    app.update();
    assert!(app.world().get::<WidgetryComboBox<u32>>(root).is_some());
    let popup = app.world().get::<Children>(root).unwrap()[1];
    let list = app.world().get::<Children>(popup).unwrap()[0];
    assert_eq!(
        app.world()
            .get::<WidgetryListView<u32>>(list)
            .unwrap()
            .source(),
        source
    );
    assert_eq!(
        *app.world().get::<WidgetryListViewState>(list).unwrap(),
        WidgetryListViewState {
            selected: Some(selected),
            active: Some(selected)
        }
    );
    assert_eq!(
        app.world().get::<TextFont>(font).unwrap().font,
        FontSource::default()
    );
}

/// facade 构造合法 owned parent/dialog，经真实 Button 输入决议并回收 dialog，parent 完整保留。
#[test]
fn facade_message_box_resolves_and_releases_owned_dialog_resources() {
    let mut app = scene_app();
    app.add_plugins(WidgetryMessageBoxPlugin)
        .init_resource::<DialogResults>();
    app.add_observer(
        |event: On<WidgetryMessageBoxResultEvent>, mut results: ResMut<DialogResults>| {
            results.0.push((event.entity, event.result));
        },
    );
    let parent_root = app.world_mut().commands().spawn_scene(bsn! {
        owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
    }).id();
    app.update();
    let parent = app
        .world_mut()
        .query_filtered::<Entity, With<Window>>()
        .single(app.world())
        .unwrap();
    let parent_camera = app.world().get::<UiTargetCamera>(parent_root).unwrap().0;
    let baseline = app
        .world()
        .get::<Children>(parent_root)
        .unwrap()
        .iter()
        .collect::<Vec<_>>();
    let root = app.world_mut().commands().spawn_scene(bsn! {
        widgetry_message_box(parent, "Facade dialog", WidgetryMessageBoxButtons::Ok, bsn_list![(Text("facade body") Name("facade body"))])
    }).id();
    app.update();
    let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
    let native = match app
        .world()
        .get::<bevy::camera::RenderTarget>(camera)
        .unwrap()
    {
        bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(entity)) => *entity,
        target => panic!("dialog 应绑定 native target，实际为 {target:?}"),
    };
    let body = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, name)| name.as_str() == "facade body")
        .unwrap()
        .0;
    let button = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryButton>>()
        .single(app.world())
        .unwrap();
    press(&mut app, button);
    app.world_mut().trigger(primary_click(button));
    app.world_mut().flush();
    assert_eq!(
        app.world().resource::<DialogResults>().0,
        vec![(root, WidgetryMessageBoxResult::Ok)]
    );
    for entity in [root, native, camera, body, button] {
        assert!(app.world().get_entity(entity).is_err());
    }
    for entity in [parent_root, parent, parent_camera] {
        assert!(app.world().get_entity(entity).is_ok());
    }
    assert_eq!(
        app.world()
            .get::<Children>(parent_root)
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        baseline
    );
}
