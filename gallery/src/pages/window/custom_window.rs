const LAUNCH_WIDTH: f32 = 200.0;
const LAUNCH_HEIGHT: f32 = 40.0;
// The centered controls clear the window shell's top resize handles.
const CONTROL_HEIGHT: f32 = 22.0;
const CONTENT_PADDING: f32 = 24.0;

use crate::{
    close::GalleryIndependentWindow,
    i18n::{
        Language, LocalizedText, LocalizedWindowTitle, TextKey, WindowKey, localized, refresh_text,
    },
};
use bevy::{prelude::*, ui_widgets::Activate, window::WindowResizeConstraints};
use bevy_widgetry::{
    button::WidgetryButton,
    scene::WidgetrySceneCommandsExt,
    text::WidgetryText,
    window::{
        WidgetryWindowAction, WidgetryWindowBackground, WidgetryWindowBehavior,
        WidgetryWindowMaximized, WidgetryWindowTitle, owned_widgetry_window_with_controls,
    },
};
use bevy_widgetry_core::ui::WidgetryUiSystems;

#[derive(Component)]
struct CustomWindowDemo;

#[derive(Component)]
struct OptionsPanel;

#[derive(Component)]
struct MaximizeLabel;

#[derive(Clone, Copy)]
enum Control {
    Options,
    Minimize,
    ToggleMaximized,
    Close,
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        #CustomWindowLaunch @WidgetryButton {
            @children: { Some(Box::new(bsn_list! { @localized(TextKey::Window(WindowKey::CustomTitleBar)) WidgetryText }) as Box<dyn SceneList>) },
        }
        Node { width: px(LAUNCH_WIDTH), height: px(LAUNCH_HEIGHT), justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        on(open)
    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        refresh_maximize_label
            .after(WidgetryUiSystems::Build)
            .before(refresh_text),
    );
}

fn open(_: On<Activate>, mut commands: Commands, language: Res<Language>) {
    let title = LocalizedText::new(TextKey::Window(WindowKey::CustomTitleBar));
    let native = Window {
        title: title.render(*language),
        resolution: (720, 420).into(),
        resize_constraints: WindowResizeConstraints {
            min_width: 480.0,
            ..default()
        },
        ..default()
    };
    info!(demo = "custom_window", "打开自定义标题栏窗口");
    commands.spawn_scene_with_error_handler(bsn! {
        #CustomWindowDemo
        @owned_widgetry_window_with_controls(
            native,
            WidgetryWindowBehavior { double_click_maximize: false, ..default() },
            WidgetryWindowBackground::Theme,
            default(),
            bsn_list! {
                WidgetryWindowTitle WidgetryText
                Node { margin: UiRect::left(px(12)) }
                template(|_| Ok(Pickable::IGNORE))
            },
            bsn_list! { @controls() },
            bsn_list! { @body() },
        )
        template(|_| Ok(CustomWindowDemo))
        template(|_| Ok(GalleryIndependentWindow))
        template(move |_| Ok(LocalizedWindowTitle(title.clone())))
    });
}

fn controls() -> impl Scene {
    bsn! {
        Node { align_items: AlignItems::Center, column_gap: px(6), padding: UiRect::horizontal(px(8)) }
        template(|_| Ok(Pickable::IGNORE))
        Children [
            @control_button("CustomWindowOptions", Control::Options, bsn_list! {
                @localized(TextKey::Window(WindowKey::CustomWindowOptions)) WidgetryText
            })--
            @control_button("CustomWindowMinimize", Control::Minimize, bsn_list! {
                @localized(TextKey::Window(WindowKey::CustomWindowMinimize)) WidgetryText
            })--
            @control_button("CustomWindowMaximize", Control::ToggleMaximized, bsn_list! {
                #CustomWindowMaximizeLabel @localized(TextKey::Window(WindowKey::CustomWindowMaximize)) WidgetryText
                template(|_| Ok(MaximizeLabel))
            })--
            @control_button("CustomWindowClose", Control::Close, bsn_list! {
                @localized(TextKey::Window(WindowKey::CustomWindowClose)) WidgetryText
            })
        ]
    }
}

fn body() -> impl Scene {
    bsn! {
        Node { padding: UiRect::all(px(CONTENT_PADDING)), flex_direction: FlexDirection::Column, row_gap: px(12) }
        Children [
            @localized(TextKey::Window(WindowKey::CustomWindowHint)) WidgetryText--
            #CustomWindowOptionsPanel @localized(TextKey::Window(WindowKey::CustomWindowOptionsContent)) WidgetryText
            template(|_| Ok(OptionsPanel))
            Node { display: Display::None }
        ]
    }
}

fn control_button(name: &'static str, control: Control, label: impl SceneList) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: { Some(Box::new(label) as Box<dyn SceneList>) }, }
        template(move |_| Ok(Name::new(name)))
        Node { height: px(CONTROL_HEIGHT), min_height: px(CONTROL_HEIGHT), flex_shrink: 0.0, padding: UiRect::horizontal(px(10)), align_items: AlignItems::Center }
        on(move |event: On<Activate>, parents: Query<&ChildOf>, roots: Query<(Entity, &WidgetryWindowMaximized), With<CustomWindowDemo>>,
            children: Query<&Children>, mut panels: Query<&mut Node, With<OptionsPanel>>, mut commands: Commands| {
            let Some((root, maximized)) = parents.iter_ancestors(event.entity).find_map(|entity| roots.get(entity).ok()) else {
                return;
            };
            let action = match control {
                Control::Options => {
                    for entity in children.iter_descendants(root) {
                        if let Ok(mut panel) = panels.get_mut(entity) {
                            let open = panel.display == Display::None;
                            panel.display = if open { Display::Flex } else { Display::None };
                            info!(window_root = ?root, open, "切换自定义标题栏选项面板");
                        }
                    }
                    return;
                }
                Control::Minimize => WidgetryWindowAction::Minimize,
                Control::ToggleMaximized if maximized.is_maximized() => WidgetryWindowAction::RestoreFromMaximized,
                Control::ToggleMaximized => WidgetryWindowAction::Maximize,
                Control::Close => WidgetryWindowAction::RequestClose,
            };
            info!(window_root = ?root, ?action, "请求自定义窗口操作");
            action.queue(&mut commands, root);
        })
    }
}

fn refresh_maximize_label(
    roots: Query<
        (Entity, &WidgetryWindowMaximized),
        (With<CustomWindowDemo>, Changed<WidgetryWindowMaximized>),
    >,
    children: Query<&Children>,
    mut labels: Query<&mut LocalizedText, With<MaximizeLabel>>,
) {
    for (root, maximized) in &roots {
        let key = if maximized.is_maximized() {
            WindowKey::CustomWindowRestore
        } else {
            WindowKey::CustomWindowMaximize
        };
        for entity in children.iter_descendants(root) {
            if let Ok(mut label) = labels.get_mut(entity) {
                *label = LocalizedText::new(TextKey::Window(key));
            }
        }
    }
}
