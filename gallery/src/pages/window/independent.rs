use super::{DemoText, WindowDemoSection};
use bevy::app::Propagate;
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{
    button::WidgetryButton,
    style::{ForegroundColor, ThemeMode},
    window::{WidgetryWindowControlsConfig, owned_widgetry_window},
};

/// 保留独立窗口入口及其原有内容与交互。
pub(super) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(WindowDemoSection))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().window_border)))
        Node { width: percent(100), border: UiRect::top(px(1)), padding: UiRect::top(px(12)), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text("Independent Window"),
            (@WidgetryButton
                Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                on(open_window)
                Children [Text("Open Window")]),
        ]
    }
}

/// owned_widgetry_window 统一管理专用窗口和相机，按钮示例可更新自身文本。
fn open_window(_event: On<Activate>, mut commands: Commands) {
    info!("打开独立窗口");
    commands.spawn_scene(bsn! {
        owned_widgetry_window(Window { title: "Window Demo".into(), resolution: (640, 400).into(), ..default() }, WidgetryWindowControlsConfig::default(),
            bsn_list![(
                demo_text()
                Node { padding: UiRect::left(px(12)), align_items: AlignItems::Center }
                Children [(Text("Window Demo") template(|_| Ok(Pickable::IGNORE)))]
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
    });
}

/// 初始化普通文本颜色；交互控件自行提供主题前景色。
fn demo_text() -> impl Scene {
    bsn! {
        template(|_| Ok(DemoText))
        template(|_| Ok(Pickable::IGNORE))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
    }
}

/// 用可见文本反馈确认新窗口内容区的按钮能够正常交互。
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
