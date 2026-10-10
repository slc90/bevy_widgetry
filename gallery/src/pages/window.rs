mod file_dialog;
mod independent;
mod message_box;

use bevy::prelude::*;
use bevy_widgetry::message_box::WidgetryMessageBoxPlugin;

pub(crate) struct WindowDemoPlugin;

#[derive(Component)]
struct DemoText;

pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(DemoText))
        Node { width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, padding: UiRect::all(px(24)), row_gap: px(24) }
        Children [@independent::scene()-- @message_box::scene()-- @file_dialog::scene()]
    }
}

impl Plugin for WindowDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((WidgetryMessageBoxPlugin, file_dialog::FileDialogDemoPlugin))
            .add_observer(message_box::on_message_box_result);
    }
}
