use super::DemoText;
use crate::assets::GalleryImage;
use bevy::app::Propagate;
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
use bevy_widgetry::theme::WidgetryThemeMode;
use bevy_widgetry::{
    button::WidgetryButton,
    style::ForegroundColor,
    window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
        WidgetryWindowImageMode, owned_widgetry_window,
    },
};

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text("Independent Window"),
            (#WindowThemeButton @WidgetryButton
                Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                on(|_: On<Activate>, commands: Commands| open_window(commands, WidgetryWindowBackground::Theme, "Theme".into(), "WindowThemeDemo".into()))
                Children [Text("Theme")]),
            image_buttons(GalleryImage::WindowBackground1, "Background 1", "WindowBackground1"),
            image_buttons(GalleryImage::WindowBackground2, "Background 2", "WindowBackground2"),
        ]
    }
}

fn image_buttons(image: GalleryImage, label: &'static str, prefix: &'static str) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text(label),
            (Node { column_gap: px(12) } Children [
                image_button(image, WidgetryWindowImageMode::Stretch, 1.0, "Stretch", prefix, "Stretch"),
                image_button(image, WidgetryWindowImageMode::Cover, 1.0, "Cover", prefix, "Cover"),
                image_button(image, WidgetryWindowImageMode::Cover, 0.5, "Cover 50%", prefix, "Cover50"),
            ]),
        ]
    }
}

fn image_button(
    image: GalleryImage,
    mode: WidgetryWindowImageMode,
    opacity: f32,
    label: &'static str,
    prefix: &'static str,
    suffix: &'static str,
) -> impl Scene {
    let name = format!("{prefix}{suffix}");
    let button_name = format!("{name}Button");
    let title = format!(
        "Background {} - {label}",
        if matches!(image, GalleryImage::WindowBackground1) {
            1
        } else {
            2
        }
    );
    bsn! {
        @WidgetryButton
        template(move |_| Ok(Name::new(button_name.clone())))
        Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(move |_: On<Activate>, commands: Commands, assets: Res<AssetServer>| {
            let background = WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
                image: assets.load(image.path()), mode, opacity,
            });
            open_window(commands, background, title.clone(), name.clone());
        })
        Children [Text(label)]
    }
}

fn open_window(
    mut commands: Commands,
    background: WidgetryWindowBackground,
    title: String,
    name: String,
) {
    info!(window = %title, "打开独立窗口");
    commands.spawn_scene_with_error_handler(bsn! {
        owned_widgetry_window(Window { title: title.clone(), resolution: (640, 400).into(), ..default() }, WidgetryWindowControlsConfig::default(), background,
            bsn_list![(
                demo_text()
                Node { padding: UiRect::left(px(12)), align_items: AlignItems::Center }
                Children [(Text(title) template(|_| Ok(Pickable::IGNORE)))]
            )],
            bsn_list![(
                demo_text()
                Node { padding: UiRect::all(px(24)), flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
                Children [
                    Text("This is an independent Widgetry window."),
                    (
                        @WidgetryButton
                        on(on_demo_button)
                        Children [Text("Click me")]
                    ),
                ]
            )])
        template(move |_| Ok(Name::new(name.clone())))
    });
}

fn demo_text() -> impl Scene {
    bsn! {
        template(|_| Ok(DemoText))
        template(|_| Ok(Pickable::IGNORE))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<WidgetryThemeMode>().colors().text.normal.foreground))))
    }
}

pub(super) fn on_demo_button(
    event: On<Activate>,
    children: Query<&Children>,
    mut texts: Query<&mut Text>,
) {
    info!(demo = "window", entity = ?event.entity, "激活窗口内容按钮示例");
    for child in children.iter_descendants(event.entity) {
        if let Ok(mut text) = texts.get_mut(child) {
            **text = "Clicked!".into();
        }
    }
}
