use super::independent::on_demo_button;
use crate::i18n::WindowKey;
use crate::i18n::{Language, LocalizedText, LocalizedWindowTitle, TextKey, localized, tr};
use bevy::{prelude::*, ui_widgets::Activate, window::PrimaryWindow};
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
use bevy_widgetry::{
    button::WidgetryButton,
    message_box::{WidgetryMessageBoxButtons, WidgetryMessageBoxResultEvent, widgetry_message_box},
};

#[derive(Component)]
struct MessageBoxDemo(WidgetryMessageBoxButtons);

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            @localized(TextKey::Window(WindowKey::MessageBox)) bevy_widgetry::text::WidgetryText--
            Node { column_gap: px(12) } Children [
                @message_box_demo_button(TextKey::Window(WindowKey::Ok), WidgetryMessageBoxButtons::Ok)--
                @message_box_demo_button(TextKey::Window(WindowKey::YesNo), WidgetryMessageBoxButtons::YesNo)--
                @message_box_demo_button(TextKey::Window(WindowKey::YesNoCancel), WidgetryMessageBoxButtons::YesNoCancel)
            ]
        ]
    }
}

fn message_box_demo_button(label: TextKey, buttons: WidgetryMessageBoxButtons) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(label) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        template(move |_| Ok(MessageBoxDemo(buttons)))
        Node { height: px(40), padding: UiRect::axes(px(16), px(6)), align_items: AlignItems::Center }
        on(open_message_box)

    }
}

fn open_message_box(
    event: On<Activate>,
    demos: Query<&MessageBoxDemo>,
    parent: Single<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
    language: Res<Language>,
) {
    let Ok(demo) = demos.get(event.entity) else {
        return;
    };
    info!(buttons = ?demo.0, "打开 MessageBox");
    commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_message_box(*parent, tr(*language, TextKey::Window(WindowKey::MessageBoxDemo)), demo.0, Default::default(),  bsn_list!{
            @localized(TextKey::Window(WindowKey::ChooseAResultBelow)) bevy_widgetry::text::WidgetryText--
            @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(TextKey::Window(WindowKey::ContentButtonKeepsDialogOpen)) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
                Node { align_self: AlignSelf::Start }
                on(on_demo_button)

        })
        template(|_| Ok(LocalizedWindowTitle(LocalizedText::new(TextKey::Window(WindowKey::MessageBoxDemo)))))
    });
}

pub(super) fn on_message_box_result(event: On<WidgetryMessageBoxResultEvent>) {
    info!(entity = ?event.entity, result = ?event.result, "WidgetryMessageBox 返回结果");
}
