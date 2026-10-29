use super::SplashSession;
use crate::i18n::{TextKey, localized};
use bevy::prelude::*;
use bevy::ui_widgets::Button;

#[derive(Component)]
pub(super) struct SplashSkip;

pub(super) fn install(app: &mut App) {
    app.add_observer(click)
        .add_observer(over)
        .add_observer(out)
        .add_observer(press)
        .add_observer(release)
        .add_observer(cancel);
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("SplashSkip")
        template(|_| Ok(SplashSkip))
        Button
        Visibility::Hidden
        ZIndex(1)
        Node {
            position_type: PositionType::Absolute, right: px(24), top: px(24),
            width: px(72), height: px(30),
            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
            border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(5)),
        }
        BackgroundColor({Color::NONE})
        template(|_| Ok(BorderColor::all(border(0.28))))
        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
        Children [
            Name("SplashSkipLabel")
            @localized(TextKey::SplashSkip)
            TextFont { font_size: bevy::text::FontSize::Px(14.0) }
            TextColor({Color::srgba(0.38, 0.32, 0.46, 0.9)})
            template(|_| Ok(Pickable::IGNORE))
        ]
    }
}

fn click(
    mut event: On<PointerClick>,
    buttons: Query<(), With<SplashSkip>>,
    session: Option<ResMut<SplashSession>>,
) {
    if event.button != PointerButton::Primary || !buttons.contains(event.entity) {
        return;
    }
    event.propagate(false);
    if let Some(mut session) = session
        && !session.finish_requested
    {
        session.finish_requested = true;
        info!("跳过 Splash，等待 Gallery 安全交接");
    }
}

fn border(alpha: f32) -> Color {
    Color::srgba(0.55, 0.49, 0.65, alpha)
}

fn visual(
    entity: Entity,
    background: f32,
    outline: f32,
    buttons: &mut Query<(&mut BackgroundColor, &mut BorderColor), With<SplashSkip>>,
) {
    if let Ok((mut fill, mut edge)) = buttons.get_mut(entity) {
        fill.0 = Color::srgba(0.55, 0.49, 0.65, background);
        *edge = BorderColor::all(border(outline));
    }
}

fn over(
    event: On<PointerOver>,
    mut buttons: Query<(&mut BackgroundColor, &mut BorderColor), With<SplashSkip>>,
) {
    visual(event.entity, 0.035, 0.48, &mut buttons);
}
fn out(
    event: On<PointerOut>,
    mut buttons: Query<(&mut BackgroundColor, &mut BorderColor), With<SplashSkip>>,
) {
    visual(event.entity, 0.0, 0.28, &mut buttons);
}
fn press(
    event: On<PointerPress>,
    mut buttons: Query<(&mut BackgroundColor, &mut BorderColor), With<SplashSkip>>,
) {
    if event.button == PointerButton::Primary {
        visual(event.entity, 0.07, 0.65, &mut buttons);
    }
}
fn release(
    event: On<PointerRelease>,
    mut buttons: Query<(&mut BackgroundColor, &mut BorderColor), With<SplashSkip>>,
) {
    if event.button == PointerButton::Primary {
        visual(event.entity, 0.0, 0.28, &mut buttons);
    }
}
fn cancel(
    event: On<PointerCancel>,
    mut buttons: Query<(&mut BackgroundColor, &mut BorderColor), With<SplashSkip>>,
) {
    visual(event.entity, 0.0, 0.28, &mut buttons);
}
