use super::DemoText;
use crate::assets::GalleryImage;
use crate::i18n::{
    Language, LocalizedText, LocalizedWindowTitle, TextKey, localized, localized_binding,
};
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
use bevy_widgetry::{
    button::WidgetryButton,
    window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, WidgetryWindowImageBackground,
        WidgetryWindowImageMode, owned_widgetry_window,
    },
};

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            @localized(TextKey::IndependentWindow) bevy_widgetry::text::WidgetryText--
            #WindowThemeButton @WidgetryButton
                Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                on(|_: On<Activate>, commands: Commands, language: Res<Language>| open_window(commands, WidgetryWindowBackground::Theme, LocalizedText::new(TextKey::Theme), *language, "WindowThemeDemo".into()))
                Children [@localized(TextKey::Theme) bevy_widgetry::text::WidgetryText]--
            @image_buttons(GalleryImage::WindowBackground1,TextKey::Background1, "WindowBackground1")--
            @image_buttons(GalleryImage::WindowBackground2,TextKey::Background2, "WindowBackground2")
        ]
    }
}

fn image_buttons(image: GalleryImage, label: TextKey, prefix: &'static str) -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            @localized(label) bevy_widgetry::text::WidgetryText--
            Node { column_gap: px(12) } Children [
                @image_button(image, WidgetryWindowImageMode::Stretch, 1.0,TextKey::Stretch, prefix, "Stretch")--
                @image_button(image, WidgetryWindowImageMode::Cover, 1.0,TextKey::Cover, prefix, "Cover")--
                @image_button(image, WidgetryWindowImageMode::Cover, 0.5,TextKey::Cover50, prefix, "Cover50")
            ]
        ]
    }
}

fn image_button(
    image: GalleryImage,
    mode: WidgetryWindowImageMode,
    opacity: f32,
    label: TextKey,
    prefix: &'static str,
    suffix: &'static str,
) -> impl Scene {
    let name = format!("{prefix}{suffix}");
    let button_name = format!("{name}Button");
    let title = LocalizedText::with_args(
        TextKey::WindowImageTitle,
        vec![(
            "index",
            if matches!(image, GalleryImage::WindowBackground1) {
                "1"
            } else {
                "2"
            }
            .into(),
        )],
    )
    .with_key_arg("mode", label);
    bsn! {
        @WidgetryButton
        template(move |_| Ok(Name::new(button_name.clone())))
        Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(move |_: On<Activate>, commands: Commands, assets: Res<AssetServer>, language: Res<Language>| {
            let background = WidgetryWindowBackground::Image(WidgetryWindowImageBackground {
                image: assets.load(image.path()), mode, opacity,
            });
            open_window(commands, background, title.clone(), *language, name.clone());
        })
        Children [@localized(label) bevy_widgetry::text::WidgetryText]
    }
}

fn open_window(
    mut commands: Commands,
    background: WidgetryWindowBackground,
    title: LocalizedText,
    language: Language,
    name: String,
) {
    info!(window = %title.render(Language::English), "打开独立窗口");
    commands.spawn_scene_with_error_handler(bsn! {
        @owned_widgetry_window(Window { title: title.render(language), resolution: (640, 400).into(), ..default() }, WidgetryWindowControlsConfig::default(), background, Default::default(),
            bsn_list!{
                @demo_text()
                Node { padding: UiRect::left(px(12)), align_items: AlignItems::Center }
                Children [@localized_binding(title.clone()) bevy_widgetry::text::WidgetryText template(|_| Ok(Pickable::IGNORE))]
            },
            bsn_list!{
                @demo_text()
                Node { padding: UiRect::all(px(24)), flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
                Children [
                    @localized(TextKey::ThisIsAnIndependentWidgetryWindow) bevy_widgetry::text::WidgetryText--

                        @WidgetryButton
                        on(on_demo_button)
                        Children [@localized(TextKey::ClickMe) bevy_widgetry::text::WidgetryText]

                ]
            })
        template(move |_| Ok(Name::new(name.clone())))
        template(move |_| Ok(LocalizedWindowTitle(title.clone())))
    });
}

fn demo_text() -> impl Scene {
    bsn! {
        template(|_| Ok(DemoText))
        template(|_| Ok(Pickable::IGNORE))
    }
}

pub(super) fn on_demo_button(
    event: On<Activate>,
    children: Query<&Children>,
    mut texts: Query<&mut LocalizedText>,
) {
    info!(demo = "window", entity = ?event.entity, "激活窗口内容按钮示例");
    for child in children.iter_descendants(event.entity) {
        if let Ok(mut text) = texts.get_mut(child) {
            *text = LocalizedText::new(TextKey::Clicked);
        }
    }
}
