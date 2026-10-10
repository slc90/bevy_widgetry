//! Coverage Map：本文件负责公共 API 的 Scene 构造、state 消费、字体策略、Plugin 组合与 MessageBox/FileDialog lifecycle。
//! focus.rs 负责跨 Widget 的真实 pointer/keyboard focus 归属和隐藏 Popup 输入隔离。
//! State：构造待执行/成功/失败、各 Widget 的 selection 和 dialog 未决议/已关闭。
//! Stimuli：BSN 构造、注册 Plugin/renderer、公开 state API、Button 输入与关闭通知。
//! Guards：合法 source/parent/config。
//! 构造失败交给宿主 error handler。
//! Transitions：构造形成可查询 Widget。
//! state API 更新 selection。
//! 有效 Button 输入决议并关闭 dialog。
//! Invariants：入口均来自 facade，同一 source 的多个 ListView 保持独立 state。
//! 失败不残留预约 root。
//! Couplings：Plugin 组合不覆盖调用方字体策略。
//! MessageBox 关闭只回收自有资源。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::expect_used, clippy::unwrap_used)]
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
use bevy_widgetry::file_dialog::*;
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
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
use bevy_widgetry::scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
    WidgetryScrollAreaContent, WidgetryScrollAreaPlugin, WidgetryScrollAreaProps,
    WidgetryScrollAreaViewport, WidgetryScrollIntoView,
};
use bevy_widgetry::style::WidgetryAppExt;
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry::text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPlugin,
};
use bevy_widgetry::theme::{
    WIDGETRY_DARK_THEME, WIDGETRY_LIGHT_THEME, WidgetryTheme, WidgetryThemeMode,
    WidgetryThemePlugin,
};
use bevy_widgetry::tooltip::{
    TooltipContentFactory, WidgetryTooltip, WidgetryTooltipPlugin, WidgetryTooltipProps,
};
use bevy_widgetry::tree::{
    WidgetryTreeAppExt, WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeRenderer,
    WidgetryTreeView, WidgetryTreeVisibleItem,
};
use bevy_widgetry::window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
    WidgetryWindowImageMode, WidgetryWindowPlugin, owned_widgetry_window, widgetry_window,
};
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
use bevy_widgetry_test_utils::{press, primary_click, scene_app};

#[test]
fn file_dialog_facade_alone_constructs_all_modes_and_owned_result_lifecycle() {
    let mut app = scene_app();
    bevy_widgetry_test_utils::add_ui_plugins(&mut app);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..default()
    });
    app.add_plugins(WidgetryFileDialogPlugin);
    for mode in [
        WidgetryFileDialogMode::PickFile,
        WidgetryFileDialogMode::PickFiles,
        WidgetryFileDialogMode::PickDirectory,
        WidgetryFileDialogMode::PickDirectories,
        WidgetryFileDialogMode::SaveFile,
    ] {
        let root=app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: {mode}, @window: {Some(WidgetryFileDialogWindow::default())} } }).unwrap().id();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .mode(),
            mode
        );
        let native = bevy_widgetry::window::widgetry_window_target(app.world(), root).unwrap();
        let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
        assert!(app.world().get::<Window>(native).is_some());
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
        app.update();
        for entity in [root, native, camera] {
            assert!(app.world().get_entity(entity).is_err());
        }
    }
}

#[test]
fn invalid_widget_scenes_reach_host_error_handler() {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.add_plugins((
        WidgetryListViewPlugin,
        WidgetryComboBoxPlugin,
        WidgetryTooltipPlugin,
        bevy_widgetry::tree::WidgetryTreePlugin,
    ));
    let mut commands = app.world_mut().commands();
    let roots = [
        commands
            .spawn_scene_with_error_handler(bsn! { @WidgetryListView::<String> })
            .id(),
        commands
            .spawn_scene_with_error_handler(bsn! { @WidgetryComboBox::<String> })
            .id(),
        commands
            .spawn_scene_with_error_handler(bsn! { @WidgetryTreeView })
            .id(),
        commands
            .spawn_scene_with_error_handler(bsn! { @WidgetryTooltip })
            .id(),
    ];
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| app.world_mut().flush()));
    let errors = errors.take();
    assert_eq!(errors.len(), roots.len());
    assert!(
        errors
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert!(
        roots
            .iter()
            .all(|root| app.world().get_entity(*root).is_err())
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::Level::ERROR)
            .count(),
        roots.len()
    );
}

#[derive(Resource, Default)]
struct DialogResults(Vec<(Entity, WidgetryMessageBoxResult)>);

struct FileEntry {
    name: String,
}

#[derive(Component)]
struct TreeEntry(String);

#[test]
fn tree_consumer_can_register_renderers_and_use_entity_selection_through_facade() {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<UiScale>();
    app.register_renderer::<TreeEntry>(WidgetryTreeRenderer::new(|_, entry: &TreeEntry| {
        bsn_list! {Text({ entry.0.clone() }) bevy_widgetry_core::text::WidgetryText}
    }))
    .unwrap();
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
    assert!(WidgetryTreeModel::select(app.world_mut(), source, Some(node)).unwrap());
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

#[test]
fn generic_api_is_available_through_facade() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryListViewPlugin)
        .register_widgetry_list_view::<FileEntry>()
        .unwrap()
        .register_widgetry_list_view::<FileEntry>()
        .unwrap()
        .register_widgetry_list_view::<u32>()
        .unwrap();
    let mut model = WidgetryListModel::default();
    let id = model
        .push(FileEntry {
            name: "report".into(),
        })
        .unwrap();
    let source = app.world_mut().spawn(model).id();
    let renderer = WidgetryListViewRenderer::new(
        |index, entry: &FileEntry| bsn_list! {Text(format!("{index}: {}", entry.name)) bevy_widgetry_core::text::WidgetryText},
    );
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
            @renderer: {WidgetryListViewRenderer::new(|_, value: &u32| bsn_list!{Text({value.to_string()}) bevy_widgetry_core::text::WidgetryText})},
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
            @children: bsn_list!{Node { width: px(120), height: px(160) }},
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

#[test]
fn radio_group_scene_api_is_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryRadioGroupPlugin);
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryRadioGroup
        Children [@WidgetryRadioOption Children [Text("Low") bevy_widgetry_core::text::WidgetryText]-- @WidgetryRadioOption Children [Text("High") bevy_widgetry_core::text::WidgetryText]]
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

#[test]
fn facade_public_types_are_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.register_widgetry_combo_box::<String>().unwrap();
    let _ = WidgetryComboBoxPlugin;
    let props = WidgetryComboBoxProps::<String>::default();
    assert_eq!(props.source, Entity::PLACEHOLDER);
    assert_eq!(props.item_height, 32.0);
    assert_eq!(props.max_visible_items, 8);
    assert!(std::mem::size_of::<WidgetryComboBox<String>>() > 0);
    let _ = WidgetryText;
    let _ = bevy_widgetry::style::z_index::TOOLTIP;
}

#[test]
fn tooltip_scene_api_is_usable() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.init_resource::<bevy::picking::hover::HoverMap>()
        .add_plugins(WidgetryTooltipPlugin);
    let _ = WidgetryTooltipProps::default();
    let anchor = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{Node-- Text("Details") bevy_widgetry_core::text::WidgetryText})} }
        })
        .unwrap()
        .id();
    assert!(app.world().get::<WidgetryTooltip>(anchor).is_some());
}

#[test]
fn theme_command_reports_missing_mode_at_execution_without_notifying() {
    let mut app = App::new();
    app.add_plugins(WidgetryThemePlugin)
        .init_resource::<ThemeNotifications>();
    app.add_observer(
        |_: On<bevy_widgetry::theme::WidgetryThemeChanged>,
         mut seen: ResMut<ThemeNotifications>| {
            seen.0 += 1;
        },
    );
    app.set_error_handler(ErrorCapture::handler());
    let mut queue = bevy::ecs::world::CommandQueue::default();
    WidgetryThemeMode::set(
        &mut Commands::new(&mut queue, app.world()),
        WidgetryThemeMode::Light,
    );
    app.world_mut().remove_resource::<WidgetryThemeMode>();
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    errors.run(|| logs.run(|| queue.apply(app.world_mut())));
    let errors = errors.take();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
    assert!(errors[0].to_string().contains("WidgetryThemeMode"));
    assert!(logs.records().iter().any(|entry| {
        entry.level == bevy::log::Level::ERROR
            && entry
                .fields
                .get("message")
                .is_some_and(|message| message.contains("WidgetryThemeMode"))
    }));
    assert_eq!(app.world().resource::<ThemeNotifications>().0, 0);
    assert!(!app.world().contains_resource::<WidgetryThemeMode>());
}

#[derive(Resource, Default)]
struct ThemeNotifications(usize);

#[test]
fn style_theme_api_and_plugins_work_together() {
    let _: &WidgetryTheme = &WIDGETRY_DARK_THEME;
    assert_eq!(WidgetryThemeMode::Light.colors(), &WIDGETRY_LIGHT_THEME);
    let mut app = App::new();
    app.add_plugins(WidgetryThemePlugin);
    app.set_default_font(FontSource::monospace());
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    app.insert_resource(WidgetryThemeMode::Light)
        .add_plugins((WidgetryButtonPlugin, WidgetryComboBoxPlugin));
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Light)
        .expect("theme switch succeeds");
    app.update();
    assert_eq!(
        *app.world().resource::<WidgetryThemeMode>(),
        WidgetryThemeMode::Light
    );
}

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
                @colors: { bevy_widgetry::icon::WidgetryIconColorOverrides { normal: bevy_widgetry::icon::WidgetryIconStateColorOverrides { foreground: Some(Color::WHITE) }, disabled: bevy_widgetry::icon::WidgetryIconStateColorOverrides { foreground: Some(Color::WHITE) } } },
            }
        })
        .id();
    app.world_mut().flush();
    for entity in [plain, configured] {
        assert!(app.world().get::<WidgetryIcon>(entity).is_some());
        assert!(app.world().get::<Node>(entity).is_some());
    }
}

#[test]
fn window_scene_api_is_usable() {
    let _ = WidgetryWindowPlugin;
    let config = WidgetryWindowControlsConfig::default();
    assert!(config.minimize_visible && config.maximize_visible);
    let _ = bsn! {
        @widgetry_window(Entity::PLACEHOLDER, Entity::PLACEHOLDER, config, WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{Text("Body") bevy_widgetry_core::text::WidgetryText})
    };
    for mode in [
        WidgetryWindowImageMode::Stretch,
        WidgetryWindowImageMode::Cover,
    ] {
        let background = WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
            image: Handle::default(),
            mode,
            opacity: 0.5,
        });
        let _ = bsn! {
            @owned_widgetry_window(Window::default(), config, background, Default::default(),  bsn_list!{}, bsn_list!{Text("Image body") bevy_widgetry_core::text::WidgetryText})
        };
    }
}

#[test]
fn message_box_scene_api_is_usable() {
    let _ = WidgetryMessageBox;
    let _ = WidgetryMessageBoxPlugin;
    let _ = WidgetryMessageBoxResultEvent {
        entity: Entity::PLACEHOLDER,
        result: WidgetryMessageBoxResult::Ok,
    };
    let _ = bsn! {
        @widgetry_message_box(Entity::PLACEHOLDER, "Confirm", WidgetryMessageBoxButtons::YesNoCancel, Default::default(),  bsn_list!{Text("Save changes?") bevy_widgetry_core::text::WidgetryText})
    };
}

#[test]
fn button_scene_api_is_usable() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ));
    app.set_default_font(FontSource::monospace());
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
        WIDGETRY_DARK_THEME.button.normal.background
    );
}

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
        .register_widgetry_combo_box::<u32>()
        .unwrap();
    let font = app.world_mut().spawn(TextFont::default()).id();
    let mut model = WidgetryListModel::default();
    model.push(0u32).unwrap();
    let selected = model.push(1u32).unwrap();
    let source = app.world_mut().spawn(model).id();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryComboBox::<u32> {
                @source: source,
                @renderer: {WidgetryListViewRenderer::new(|_, _: &u32| bsn_list!{Node})},
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
    WidgetryComboBox::<u32>::clear_selection(&mut app.world_mut().commands(), root);
    app.world_mut().flush();
    assert_eq!(
        app.world()
            .get::<WidgetryListViewState>(list)
            .unwrap()
            .selected,
        None
    );
    assert_eq!(
        app.world().get::<TextFont>(font).unwrap().font,
        FontSource::default()
    );
}

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
        @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
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
        @widgetry_message_box(parent, "Facade dialog", WidgetryMessageBoxButtons::Ok, Default::default(),  bsn_list!{Text("facade body") bevy_widgetry_core::text::WidgetryText Name("facade body")})
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

#[test]
fn standalone_widget_plugins_and_facade_share_ui_theme_and_pointer_plugins() {
    use bevy_widgetry_core::pointer::WidgetryPointerPlugin;
    let plugins: [fn(&mut App); 15] = [
        |app| {
            app.add_plugins(WidgetryComboBoxPlugin);
        },
        |app| {
            app.add_plugins(bevy_widgetry::tree::WidgetryTreePlugin);
        },
        |app| {
            app.add_plugins(bevy_widgetry::waveform::WaveformRenderPlugin);
        },
        |app| {
            app.add_plugins(WidgetryMessageBoxPlugin);
        },
        |app| {
            app.add_plugins(WidgetryFileDialogPlugin);
        },
        |app| {
            app.add_plugins(WidgetryButtonPlugin);
        },
        |app| {
            app.add_plugins(WidgetryCheckBoxPlugin);
        },
        |app| {
            app.add_plugins(WidgetryRadioGroupPlugin);
        },
        |app| {
            app.add_plugins(WidgetryTextFieldPlugin);
        },
        |app| {
            app.add_plugins(WidgetryScrollAreaPlugin);
        },
        |app| {
            app.add_plugins(WidgetryListViewPlugin);
        },
        |app| {
            app.add_plugins(bevy_widgetry::table::WidgetryTablePlugin);
        },
        |app| {
            app.add_plugins(WidgetryTooltipPlugin);
        },
        |app| {
            app.add_plugins(WidgetryWindowPlugin);
        },
        |app| {
            app.add_plugins((
                WidgetryButtonPlugin,
                WidgetryCheckBoxPlugin,
                WidgetryRadioGroupPlugin,
                WidgetryTextFieldPlugin,
                WidgetryScrollAreaPlugin,
                WidgetryListViewPlugin,
                bevy_widgetry::table::WidgetryTablePlugin,
                WidgetryTooltipPlugin,
                WidgetryWindowPlugin,
            ));
        },
    ];
    for install in plugins {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::input_focus::InputFocusPlugin,
        ));
        app.init_asset::<Image>().init_asset::<Font>();
        app.insert_resource(WidgetryThemeMode::Light);
        assert!(!app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>());
        install(&mut app);
        assert!(app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>());
        assert!(app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>());
        assert_eq!(app.get_added_plugins::<WidgetryThemePlugin>().len(), 1);
        assert_eq!(
            *app.world().resource::<WidgetryThemeMode>(),
            WidgetryThemeMode::Light
        );
        assert!(app.is_plugin_added::<WidgetryPointerPlugin>());
        let count = app.get_added_plugins::<WidgetryPointerPlugin>().len();
        assert_eq!(count, 1);
    }
}

#[test]
fn standalone_button_pointer_input_ensures_focus_dependencies_once() {
    #[derive(Resource, Default)]
    struct Activations(usize);

    for preinstalled in [false, true] {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()));
        app.init_asset::<Image>()
            .init_asset::<Font>()
            .init_asset::<bevy::scene::ScenePatch>();
        if preinstalled {
            app.add_plugins((
                bevy::input_focus::InputFocusPlugin,
                bevy::input_focus::pointer_focus::PointerFocusPlugin,
            ));
        }
        app.add_plugins(WidgetryButtonPlugin)
            .init_resource::<Activations>()
            .add_observer(
                |_: On<bevy::ui_widgets::Activate>, mut seen: ResMut<Activations>| {
                    seen.0 += 1;
                },
            );
        app.world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow));
        let button = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryButton })
            .unwrap()
            .id();
        app.update();
        press(&mut app, button);
        assert!(app.world().get::<bevy::ui::Pressed>(button).is_some());
        app.world_mut().trigger(primary_click(button));
        app.world_mut().flush();
        assert_eq!(app.world().resource::<Activations>().0, 1);
        assert_eq!(
            app.world()
                .resource::<bevy::input_focus::InputFocus>()
                .get(),
            Some(button)
        );
        assert_eq!(
            app.get_added_plugins::<bevy::input_focus::InputFocusPlugin>()
                .len(),
            usize::from(preinstalled)
        );
        assert_eq!(
            app.get_added_plugins::<bevy::input_focus::pointer_focus::PointerFocusPlugin>()
                .len(),
            1
        );
    }
}
