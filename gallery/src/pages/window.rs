mod file_dialog;
mod independent;
mod message_box;

use bevy::{app::Propagate, prelude::*};
use bevy_widgetry::{
    message_box::MessageBoxPlugin,
    style::{ForegroundColor, ThemeChanged, ThemeMode},
};

/// 装配三个窗口示例区块及共用主题响应。
pub(crate) struct WindowDemoPlugin;

/// 标记普通文本容器，主题切换时更新继承前景色。
#[derive(Component)]
struct DemoText;

/// 标记区块顶部边框，使分隔线统一跟随主题。
#[derive(Component)]
struct WindowDemoSection;

/// 纵向装配三类窗口示例，子模块负责各自的内容与行为。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(DemoText))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node { width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, padding: UiRect::all(px(24)), row_gap: px(24) }
        Children [independent::scene(), message_box::scene(), file_dialog::scene()]
    }
}

/// 普通文本与顶部横线共用页面主题，按钮保留自身状态样式。
fn refresh_demo_theme(
    event: On<ThemeChanged>,
    mut texts: Query<&mut Propagate<ForegroundColor>, With<DemoText>>,
    mut sections: Query<&mut BorderColor, With<WindowDemoSection>>,
) {
    for mut foreground in &mut texts {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
    for mut border in &mut sections {
        *border = BorderColor::all(event.mode.colors().window_border);
    }
}

impl Plugin for WindowDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MessageBoxPlugin, file_dialog::FileDialogDemoPlugin))
            .add_observer(refresh_demo_theme)
            .add_observer(message_box::on_message_box_result);
    }
}
