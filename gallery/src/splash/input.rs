use super::{SplashSession, presentation::SplashPresentation};
use bevy::ecs::message::MessageMutator;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::InputFocusSystems;
use bevy::prelude::*;
use bevy::window::Ime;

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PreUpdate,
        isolate
            .after(bevy::input::InputSystems)
            .before(InputFocusSystems::Dispatch)
            .run_if(resource_exists::<SplashSession>),
    );
}

fn isolate(
    session: Res<SplashSession>,
    presentation: Res<SplashPresentation>,
    mut keyboard: MessageMutator<KeyboardInput>,
    mut ime: MessageMutator<Ime>,
    mut codes: ResMut<ButtonInput<KeyCode>>,
    mut keys: ResMut<ButtonInput<Key>>,
) {
    // Window 的 keyboard dispatch 自行处理 Tab，单纯清空 InputFocus 仍会选中底层控件。
    // 保留 message id，把主窗口输入路由到非 Window 的覆盖节点，使 dispatch 和 IME 拒绝该 target。
    for input in keyboard.read() {
        if input.window == presentation.window {
            codes.reset(input.key_code);
            keys.reset(input.logical_key.clone());
            input.window = session.overlay;
        }
    }
    for input in ime.read() {
        let window = match input {
            Ime::Preedit { window, .. }
            | Ime::Commit { window, .. }
            | Ime::Enabled { window }
            | Ime::Disabled { window } => window,
        };
        if *window == presentation.window {
            *window = session.overlay;
        }
    }
}
