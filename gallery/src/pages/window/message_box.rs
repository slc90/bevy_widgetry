use super::independent::on_demo_button;
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
            Text("MessageBox") bevy_widgetry::text::WidgetryText--
            Node { column_gap: px(12) } Children [
                @message_box_demo_button("OK", WidgetryMessageBoxButtons::Ok)--
                @message_box_demo_button("Yes / No", WidgetryMessageBoxButtons::YesNo)--
                @message_box_demo_button("Yes / No / Cancel", WidgetryMessageBoxButtons::YesNoCancel)
            ]
        ]
    }
}

fn message_box_demo_button(label: &'static str, buttons: WidgetryMessageBoxButtons) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(MessageBoxDemo(buttons)))
        Node { height: px(40), padding: UiRect::axes(px(16), px(6)), align_items: AlignItems::Center }
        on(open_message_box)
        Children [Text(label) bevy_widgetry::text::WidgetryText]
    }
}

fn open_message_box(
    event: On<Activate>,
    demos: Query<&MessageBoxDemo>,
    parent: Single<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Ok(demo) = demos.get(event.entity) else {
        return;
    };
    info!(buttons = ?demo.0, "打开 MessageBox");
    commands.spawn_scene_with_error_handler(bsn! {
        @widgetry_message_box(*parent, "MessageBox Demo", demo.0, Default::default(),  bsn_list!{
            Text("Choose a result below.") bevy_widgetry::text::WidgetryText--
            @WidgetryButton
                Node { align_self: AlignSelf::Start }
                on(on_demo_button)
                Children [Text("Content button (keeps dialog open)") bevy_widgetry::text::WidgetryText]
        })
    });
}

pub(super) fn on_message_box_result(event: On<WidgetryMessageBoxResultEvent>) {
    info!(entity = ?event.entity, result = ?event.result, "WidgetryMessageBox 返回结果");
}
