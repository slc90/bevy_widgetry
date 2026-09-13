use bevy::app::Propagate;
use bevy::{prelude::*, ui_widgets::Activate, window::PrimaryWindow};
use bevy_widgetry::{
    button::StyledButton,
    message_box::{MessageBoxButtons, MessageBoxPlugin, MessageBoxResultEvent, message_box},
    style::{ForegroundColor, ThemeChanged, ThemeMode},
    window::{WindowControlsConfig, owned_window},
};

/// 装配普通窗口、MessageBox 展示及文本主题响应。
pub(crate) struct WindowDemoPlugin;

/// 页面入口记录要展示的按钮组合，激活时传给 MessageBox。
#[derive(Component)]
struct MessageBoxDemo(MessageBoxButtons);

/// 标记演示窗口的普通文本容器，主题切换时更新继承前景色。
#[derive(Component)]
struct DemoText;

/// 纵向展示独立窗口和三种结果组合，保留页面边缘留白。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, padding: UiRect::all(px(24)), row_gap: px(16) }
        Children [
            (Node { column_gap: px(12) } Children [(
                template(|_| Ok(StyledButton))
                Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                on(open_window)
                Children [Text("Open Window")]
            )]),
            (Node { column_gap: px(12) } Children [
                message_box_demo_button("OK", MessageBoxButtons::Ok),
                message_box_demo_button("Yes / No", MessageBoxButtons::YesNo),
                message_box_demo_button("Yes / No / Cancel", MessageBoxButtons::YesNoCancel),
            ]),
        ]
    }
}

/// owned_window 统一管理专用窗口和相机，按钮示例可更新自身文本。
fn open_window(_event: On<Activate>, mut commands: Commands) {
    commands.spawn_scene(bsn! {
        owned_window(Window { title: "Window Demo".into(), resolution: (640, 400).into(), ..default() }, WindowControlsConfig::default(),
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
                        template(|_| Ok(StyledButton))
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
fn on_demo_button(event: On<Activate>, children: Query<&Children>, mut texts: Query<&mut Text>) {
    for child in children.iter_descendants(event.entity) {
        if let Ok(mut text) = texts.get_mut(child) {
            **text = "Clicked!".into();
        }
    }
}

/// 复用三个入口的样式与激活处理，组合值保持在入口实体上。
fn message_box_demo_button(label: &'static str, buttons: MessageBoxButtons) -> impl Scene {
    bsn! {
        template(|_| Ok(StyledButton))
        template(move |_| Ok(MessageBoxDemo(buttons)))
        Node { height: px(40), padding: UiRect::axes(px(16), px(6)), border: UiRect::all(px(1)), align_items: AlignItems::Center }
        on(open_message_box)
        Children [Text(label)]
    }
}

/// 三种组合均以 Gallery 主原生窗口为父，正文普通按钮只更新自身文本。
fn open_message_box(
    event: On<Activate>,
    demos: Query<&MessageBoxDemo>,
    parent: Single<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    let Ok(demo) = demos.get(event.entity) else {
        return;
    };
    commands.spawn_scene(bsn! {
        message_box(*parent, "MessageBox Demo", demo.0, bsn_list![
            Text("Choose a result below."),
            (template(|_| Ok(StyledButton))
                Node { align_self: AlignSelf::Start, padding: UiRect::axes(px(12), px(6)), border: UiRect::all(px(1)) }
                on(on_demo_button)
                Children [Text("Content button (keeps dialog open)")]),
        ])
    });
}

/// 记录显式结果便于人工核对；系统关闭没有结果通知。
fn on_message_box_result(event: On<MessageBoxResultEvent>) {
    info!(entity = ?event.entity, result = ?event.result, "MessageBox 返回结果");
}

/// 演示普通文本跟随应用主题，保留按钮自身的状态样式。
fn refresh_demo_theme(
    event: On<ThemeChanged>,
    mut texts: Query<&mut Propagate<ForegroundColor>, With<DemoText>>,
) {
    for mut foreground in &mut texts {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Plugin for WindowDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MessageBoxPlugin)
            .add_observer(refresh_demo_theme)
            .add_observer(on_message_box_result);
    }
}
