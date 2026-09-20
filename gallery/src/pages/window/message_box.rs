use super::{WindowDemoSection, independent::on_demo_button};
use bevy::{prelude::*, ui_widgets::Activate, window::PrimaryWindow};
use bevy_widgetry::{
    button::WidgetryButton,
    message_box::{WidgetryMessageBoxButtons, WidgetryMessageBoxResultEvent, widgetry_message_box},
    style::ThemeMode,
};

/// 页面入口记录要展示的 button 组合，Activate 时传给 WidgetryMessageBox。
#[derive(Component)]
struct MessageBoxDemo(WidgetryMessageBoxButtons);

/// 展示项目 WidgetryMessageBox 的三种固定结果组合。
pub(super) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(WindowDemoSection))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().window_border)))
        Node { width: percent(100), border: UiRect::top(px(1)), padding: UiRect::top(px(12)), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text("MessageBox"),
            (Node { column_gap: px(12) } Children [
                message_box_demo_button("OK", WidgetryMessageBoxButtons::Ok),
                message_box_demo_button("Yes / No", WidgetryMessageBoxButtons::YesNo),
                message_box_demo_button("Yes / No / Cancel", WidgetryMessageBoxButtons::YesNoCancel),
            ]),
        ]
    }
}

/// 复用三个入口的 style 与 Activate 处理，组合值保持在入口 entity 上。
fn message_box_demo_button(label: &'static str, buttons: WidgetryMessageBoxButtons) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(MessageBoxDemo(buttons)))
        Node { height: px(40), padding: UiRect::axes(px(16), px(6)), align_items: AlignItems::Center }
        on(open_message_box)
        Children [Text(label)]
    }
}

/// 三种组合均以 Gallery 的主 native window 为 parent，正文普通 button 只更新自身文本。
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
    commands.spawn_scene(bsn! {
        widgetry_message_box(*parent, "MessageBox Demo", demo.0, bsn_list![
            Text("Choose a result below."),
            (@WidgetryButton
                Node { align_self: AlignSelf::Start }
                on(on_demo_button)
                Children [Text("Content button (keeps dialog open)")]),
        ])
    });
}

/// 记录显式结果便于人工核对；操作系统关闭没有结果通知。
pub(super) fn on_message_box_result(event: On<WidgetryMessageBoxResultEvent>) {
    info!(entity = ?event.entity, result = ?event.result, "WidgetryMessageBox 返回结果");
}
