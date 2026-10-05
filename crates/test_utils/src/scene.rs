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

pub fn text_input_app() -> App {
    let mut app = scene_app();
    app.init_resource::<ButtonInput<Key>>()
        .init_resource::<UiScale>()
        .add_message::<Ime>()
        .add_message::<Pointer<Release>>()
        .add_plugins(EditableTextInputPlugin);
    app
}

pub fn text_edit_app() -> App {
    let mut app = text_input_app();
    add_ui_plugins(&mut app);
    spawn_ui_camera(&mut app, UVec2::splat(600), 1.0);
    app
}

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

pub fn add_keyboard_dispatch(app: &mut App) {
    app.add_message::<KeyboardInput>().add_systems(
        PreUpdate,
        bevy::input_focus::dispatch_focused_input::<KeyboardInput>
            .in_set(bevy::input_focus::InputFocusSystems::Dispatch),
    );
}

pub fn queue_key(app: &mut App, mut input: KeyboardInput) {
    // PLACEHOLDER 表示测试 fixture 的 primary window，明确 window identity 的事件保持原值。
    if input.window == Entity::PLACEHOLDER {
        input.window = app
            .world_mut()
            .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
            .single(app.world())
            .unwrap_or(input.window);
    }
    app.world_mut().write_message(input);
}

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

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;

    #[derive(Resource, Default)]
    struct Updates(usize);

    fn count_updates(mut updates: ResMut<Updates>) {
        updates.0 += 1;
    }

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

    #[test]
    fn condition_wait_reports_timeout_target() {
        let mut app = scene_app();
        let error =
            advance_until(&mut app, Duration::ZERO, "entity 42 的 Font", |_| false).unwrap_err();
        assert!(error.contains("entity 42 的 Font"));
        assert!(error.contains("0 次 update"));
    }
}
