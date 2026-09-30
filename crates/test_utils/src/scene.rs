//! 按需装配独立 headless App：轻量 Scene、完整 UI 与 keyboard dispatch 的依赖分别显式声明。
//! 输入入队和 camera 创建不推进；press_key 推进一次；条件等待仅供前置就绪和合法收敛。

use bevy::prelude::*;
use bevy::{
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput, NativeKey},
    },
    input_focus::InputFocusPlugin,
    picking::events::{Pointer, Release},
    ui_widgets::EditableTextInputPlugin,
    window::Ime,
};
use bevy_widgetry_core::WidgetryAppExt;
use std::time::{Duration, Instant};

/// 为跨 Widget 的 BSN lifecycle 测试提供 headless 的 asset、字体策略与官方 InputFocusPlugin。
/// 调用方继续装配被测 Widget plugin，不创建 native window 或 render device。
/// 不包含完整 UI pipeline 或 keyboard dispatch，按需调用 add_ui_plugins 与 add_keyboard_dispatch。
pub fn scene_app() -> App {
    let mut app = App::new();
    app.set_default_font(bevy::text::FontSource::Monospace);
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), InputFocusPlugin));
    app.init_asset::<Image>();
    app.init_asset::<bevy::scene::ScenePatch>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.add_message::<bevy::window::WindowCloseRequested>();
    app
}

/// 为 scene_app 装配真实 UI、文本、picking 和 visibility system，不创建 native window 或 render device。
/// 调用方按需提供 camera 和已加载字体，以验证新内容的首帧渲染准备。
pub fn add_ui_plugins(app: &mut App) {
    app.init_resource::<bevy::text::FontCx>()
        .init_resource::<bevy::text::ScaleCx>()
        .init_resource::<bevy::text::TextPipeline>()
        .init_resource::<bevy::input::touch::Touches>()
        .add_message::<bevy::window::WindowEvent>()
        .init_asset::<bevy::image::TextureAtlasLayout>()
        .init_asset::<Mesh>()
        .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(bevy::picking::DefaultPickingPlugins)
        .add_plugins(bevy::text::TextPlugin)
        .add_plugins(bevy::ui::UiPlugin)
        .add_plugins(bevy::camera::visibility::VisibilityPlugin);
}

/// 在 headless Scene 环境中运行官方文本输入 observer 和 focus event dispatch，不装配 render pipeline。
pub fn text_input_app() -> App {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<Key>>()
        .init_resource::<UiScale>()
        .add_message::<Ime>()
        .add_message::<Pointer<Release>>()
        .add_plugins(EditableTextInputPlugin);
    app
}

/// 在 text_input_app 上运行官方 TextPlugin 的编辑消费，供只检查文本/selection 的测试使用。
/// 使用真实 headless UI layout 和 camera，不创建 native window，不发送读取系统 clipboard 的操作。
pub fn text_edit_app() -> App {
    let mut app = text_input_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(600), 1.0);
    app
}

/// 写入一次按键并推进官方 focused-input dispatch；调用方需装配 KeyboardInput message 与 dispatch system。
pub fn press_key(app: &mut App, window: Entity, key_code: KeyCode) {
    queue_key(
        app,
        KeyboardInput {
            key_code,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        },
    );
    app.update();
}

/// 在 scene_app 上装配官方 keyboard focused-input dispatch，不推进 App。
/// 不装配 InputPlugin；原始输入的 window 保持不变，dispatch 路由遵循 Bevy 的 PrimaryWindow contract。
/// 依赖 scene_app 的 InputFocusPlugin；已装配 InputDispatchPlugin 的 App 不应重复调用。
pub fn add_keyboard_dispatch(app: &mut App) {
    app.add_message::<KeyboardInput>().add_systems(
        PreUpdate,
        bevy::input_focus::dispatch_focused_input::<KeyboardInput>
            .in_set(bevy::input_focus::InputFocusSystems::Dispatch),
    );
}

/// 仅将完整 KeyboardInput 入队，不执行 update；调用方先注册 message，再自行决定同帧批量输入的推进边界。
pub fn queue_key(app: &mut App, input: KeyboardInput) {
    app.world_mut().write_message(input);
}

/// 为 add_ui_plugins 的 headless layout 创建计算 camera，不创建 window 或 render device，不推进 App。
/// 物理尺寸和 scale factor 由调用方显式提供；不装配会重写计算信息的 CameraPlugin。
pub fn spawn_ui_camera(app: &mut App, physical_size: UVec2, scale_factor: f32) -> Entity {
    app.world_mut()
        .spawn((
            Camera2d,
            Camera {
                computed: bevy::camera::ComputedCameraValues {
                    target_info: Some(bevy::camera::RenderTargetInfo {
                        physical_size,
                        scale_factor,
                    }),
                    ..default()
                },
                viewport: Some(bevy::camera::Viewport {
                    physical_size,
                    ..default()
                }),
                ..default()
            },
        ))
        .id()
}

/// 等待前置资源就绪或合同允许的收敛，返回实际执行的 update 次数；条件已满足时不推进。
/// 超时错误包含调用方提供的目标资源或 entity 说明。不得用本 helper 证明操作的同帧保证。
pub fn advance_until(
    app: &mut App,
    timeout: Duration,
    target: &str,
    mut ready: impl FnMut(&World) -> bool,
) -> Result<usize, String> {
    let start = Instant::now();
    let mut updates = 0;
    while !ready(app.world()) {
        if start.elapsed() >= timeout {
            return Err(format!(
                "等待 {target} 超时（{timeout:?}），已推进 {updates} 次 update"
            ));
        }
        app.update();
        updates += 1;
        std::thread::yield_now();
    }
    Ok(updates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;

    /// 每次 update 的可观察计数，防止入队与等待 helper 隐式推进。
    #[derive(Resource, Default)]
    struct Updates(usize);

    /// 用真实 schedule 记录推进次数。
    fn count_updates(mut updates: ResMut<Updates>) {
        updates.0 += 1;
    }

    /// Pressed、repeat 与 Released 同帧入队，保留完整输入和顺序，不执行 update。
    #[test]
    fn queued_keyboard_batch_does_not_advance_app() {
        let mut app = scene_app();
        add_keyboard_dispatch(&mut app);
        app.init_resource::<Updates>()
            .add_systems(Update, count_updates);
        let window = app.world_mut().spawn_empty().id();
        let sequence = [
            (ButtonState::Pressed, false),
            (ButtonState::Pressed, true),
            (ButtonState::Released, false),
        ];
        for (state, repeat) in sequence {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code: KeyCode::KeyA,
                    logical_key: Key::Character("a".into()),
                    state,
                    text: Some("a".into()),
                    repeat,
                    window,
                },
            );
        }
        assert_eq!(app.world().resource::<Updates>().0, 0);
        let messages = app.world().resource::<Messages<KeyboardInput>>();
        let mut cursor = messages.get_cursor();
        let inputs: Vec<_> = cursor.read(messages).collect();
        assert_eq!(
            inputs
                .iter()
                .map(|input| (input.state, input.repeat))
                .collect::<Vec<_>>(),
            sequence
        );
        for input in inputs {
            assert_eq!(input.logical_key, Key::Character("a".into()));
            assert_eq!(input.text.as_deref(), Some("a"));
            assert_eq!(input.key_code, KeyCode::KeyA);
            assert_eq!(input.window, window);
        }
        app.update();
        assert_eq!(app.world().resource::<Updates>().0, 1);
    }

    /// 两个 window 的消息保留各自原始 identity，fixture 不擅自归一化输入。
    #[test]
    fn queued_keyboard_preserves_window_identity() {
        let mut app = scene_app();
        add_keyboard_dispatch(&mut app);
        let windows = [
            app.world_mut().spawn_empty().id(),
            app.world_mut().spawn_empty().id(),
        ];
        for window in windows {
            queue_key(
                &mut app,
                KeyboardInput {
                    key_code: KeyCode::Enter,
                    logical_key: Key::Enter,
                    state: ButtonState::Pressed,
                    text: None,
                    repeat: false,
                    window,
                },
            );
        }
        let messages = app.world().resource::<Messages<KeyboardInput>>();
        let mut cursor = messages.get_cursor();
        assert_eq!(
            cursor
                .read(messages)
                .map(|input| input.window)
                .collect::<Vec<_>>(),
            windows
        );
    }

    /// 非单位 DPI 下，camera 的逻辑 viewport 保留物理尺寸与 scale factor 的关系。
    #[test]
    fn ui_camera_preserves_size_and_scale() {
        let mut app = scene_app();
        let entity = spawn_ui_camera(&mut app, UVec2::new(800, 600), 2.0);
        let camera = app.world().get::<Camera>(entity).unwrap();
        assert_eq!(camera.physical_viewport_size(), Some(UVec2::new(800, 600)));
        assert_eq!(
            camera.logical_viewport_size(),
            Some(Vec2::new(400.0, 300.0))
        );
    }

    /// 已就绪时不推进；需要两次 update 时返回实际次数。
    #[test]
    fn condition_wait_counts_only_required_updates() {
        let mut app = scene_app();
        app.init_resource::<Updates>()
            .add_systems(Update, count_updates);
        assert_eq!(
            advance_until(&mut app, Duration::ZERO, "Updates 初始值", |world| world
                .resource::<Updates>()
                .0
                == 0)
            .unwrap(),
            0
        );
        assert_eq!(
            advance_until(
                &mut app,
                Duration::from_secs(1),
                "Updates 达到 2",
                |world| world.resource::<Updates>().0 == 2
            )
            .unwrap(),
            2
        );
    }

    /// 超时报告等待的目标和实际次数，不在零期限下暗中执行 update。
    #[test]
    fn condition_wait_reports_timeout_target() {
        let mut app = scene_app();
        let error =
            advance_until(&mut app, Duration::ZERO, "entity 42 的 Font", |_| false).unwrap_err();
        assert!(error.contains("entity 42 的 Font"));
        assert!(error.contains("0 次 update"));
    }
}
