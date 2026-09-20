use bevy::{
    app::{App, Plugin},
    input_focus::InputFocus,
    picking::{
        events::{Pointer, Press},
        pointer::PointerButton,
    },
    prelude::{On, Query, ResMut, With},
    text::EditableText,
};
use bevy_widgetry_log::widgetry_info;

/// 主 pointer 在非 EditableText 目标上 press 时清除 focus；文本间切换由官方输入 plugin 处理。
/// 不安装文本输入或 Tab navigation plugin；FocusGained / FocusLost 由应用的 InputFocusPlugin 派发。
pub struct WidgetryFocusPlugin;

/// 仅判断原始 picking 目标，避免 event bubbling 到普通 parent node 后误清除文本 focus。
fn clear_focus_on_non_editable_press(
    press: On<Pointer<Press>>,
    editable: Query<(), With<EditableText>>,
    mut focus: ResMut<InputFocus>,
) {
    if press.button == PointerButton::Primary && !editable.contains(press.original_event_target()) {
        focus.clear();
    }
}

impl Plugin for WidgetryFocusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputFocus>()
            .add_observer(clear_focus_on_non_editable_press);
        widgetry_info!("WidgetryFocusPlugin 注册完成");
    }
}
