mod file_dialog;
mod independent;
mod message_box;

use bevy::{app::Propagate, prelude::*};
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};
use bevy_widgetry::{message_box::WidgetryMessageBoxPlugin, style::ForegroundColor};

pub(crate) struct WindowDemoPlugin;

#[derive(Component)]
struct DemoText;

pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(DemoText))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<WidgetryThemeMode>().colors().text.normal.foreground))))
        Node { width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, padding: UiRect::all(px(24)), row_gap: px(24) }
        Children [independent::scene(), message_box::scene(), file_dialog::scene()]
    }
}

fn refresh_demo_theme(
    event: On<WidgetryThemeChanged>,
    mut texts: Query<&mut Propagate<ForegroundColor>, With<DemoText>>,
) {
    for mut foreground in &mut texts {
        foreground.0 = ForegroundColor(event.mode.colors().text.normal.foreground);
    }
}

impl Plugin for WindowDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((WidgetryMessageBoxPlugin, file_dialog::FileDialogDemoPlugin))
            .add_observer(refresh_demo_theme)
            .add_observer(message_box::on_message_box_result);
    }
}
