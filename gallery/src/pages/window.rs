use bevy::app::Propagate;
use bevy::{prelude::*, ui_widgets::Activate, window::WindowClosed};
use bevy_widgetry::{
    button::StyledButton,
    style::{ForegroundColor, ThemeChanged, ThemeMode},
    window::{WindowControlsConfig, widgetry_window, window},
};

/// 在示例模块内部装配专用 Camera 和文本的生命周期。
pub(crate) struct WindowDemoPlugin;

/// 此示例的相机专属一个弹窗，由 Gallery 负责释放。
#[derive(Component)]
struct DemoCamera {
    /// 关闭此窗口后相机也不再有用途。
    target_window: Entity,
}

/// 标记演示窗口的普通文本容器，主题切换时更新继承前景色。
#[derive(Component)]
struct DemoText;

/// 页面只提供一个入口，允许连续创建多个独立窗口。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(StyledButton))
        Node { width: px(160), height: px(40), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(open_window)
        Children [Text("Open Window")]
    }
}

/// 每次激活独立创建原生窗口、相机和 Window 场景，按钮示例可更新自身文本。
fn open_window(_event: On<Activate>, mut commands: Commands) {
    let target = commands
        .spawn(widgetry_window(Window {
            title: "Window Demo".into(),
            resolution: (640, 400).into(),
            ..default()
        }))
        .id();
    let camera = commands
        .spawn((
            Camera2d,
            DemoCamera {
                target_window: target,
            },
        ))
        .id();
    commands.spawn_scene(bsn! {
        window(target, camera, WindowControlsConfig::default(),
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

/// Gallery 采用一窗口一专用相机的示例所有权关系，故关闭时回收专用相机。
/// 实际业务是否清理 Camera 取决于调用方所有权，Window 控件只配置 Camera 的窗口渲染属性。
fn cleanup_window_cameras(
    mut closed: MessageReader<WindowClosed>,
    cameras: Query<(Entity, &DemoCamera)>,
    mut commands: Commands,
) {
    for event in closed.read() {
        for (entity, camera) in &cameras {
            if camera.target_window == event.window {
                commands.entity(entity).despawn();
            }
        }
    }
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
        app.add_systems(Update, cleanup_window_cameras)
            .add_observer(refresh_demo_theme);
    }
}
