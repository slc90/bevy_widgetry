use crate::{
    gallery::GalleryPage,
    i18n::{ButtonKey, TextKey, localized},
};
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry::{button::WidgetryButton, hold_button::WidgetryHoldButton, text::WidgetryText};

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct HoldDemoCount {
    activations: usize,
    underlay: usize,
}
#[derive(Component)]
struct CountText;
#[derive(Component)]
struct MainHold;
#[derive(Component)]
struct HoldParent;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Focus,
    Blur,
    Disabled,
    ParentDisabled,
    Visibility,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Update,
        (shortcuts, count_text)
            .chain()
            .run_if(in_state(GalleryPage::Button)),
    );
}

fn hold(diameter: f32, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryHoldButton {
            @diameter: diameter,
            @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(ButtonKey::HoldConfirm)) WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) } }) as Box<dyn SceneList>)},
        }
        Name(name) on(activated)
    }
}

fn control(key: ButtonKey, action: Action, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(key)) WidgetryText }) as Box<dyn SceneList>)}, }
        Name(name) template(move |_| Ok(action)) on(operate)
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("HoldButtonDemo") template(|_| Ok(HoldDemoCount::default()))
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(10), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Button(ButtonKey::HoldTitle)) WidgetryText--
            @localized(TextKey::Button(ButtonKey::HoldDescription)) WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }--
            Node { column_gap: px(20), align_items: AlignItems::Center }
            Children [
                @hold(64.0, "HoldButton64")--
                Name("HoldButtonParent") template(|_| Ok(HoldParent)) Node Children [@hold(112.0, "HoldButton112") template(|_| Ok(MainHold))]--
                @hold(160.0, "HoldButton160")--
                @hold(64.0, "HoldButtonDisabled") InteractionDisabled
            ]--
            Text("Activate = 0 / Underlay = 0") WidgetryText template(|_| Ok(CountText)) Name("HoldButtonCount")--
            Node { column_gap: px(8), row_gap: px(6), flex_wrap: FlexWrap::Wrap }
            Children [
                @control(ButtonKey::HoldFocus, Action::Focus, "HoldFocus")--
                @control(ButtonKey::HoldBlur, Action::Blur, "HoldBlur")--
                @control(ButtonKey::HoldDisable, Action::Disabled, "HoldToggleDisabled")--
                @control(ButtonKey::HoldParentDisable, Action::ParentDisabled, "HoldToggleParentDisabled")--
                @control(ButtonKey::HoldVisibility, Action::Visibility, "HoldToggleVisibility")
            ]--
            Node { width: px(160), height: px(112), flex_shrink: 0.0 }
            Children [
                @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(ButtonKey::HoldUnderlay)) WidgetryText }) as Box<dyn SceneList>)}, }
                Name("HoldUnderlay") Node { width: percent(100), height: percent(100), padding: UiRect::all(px(2)) } on(underlay)--
                @hold(112.0, "HoldPicking") Node { position_type: PositionType::Absolute, left: px(24), top: px(0) }
            ]
        ]
    }
}

fn activated(
    event: On<Activate>,
    buttons: Query<(), With<WidgetryHoldButton>>,
    mut count: Query<&mut HoldDemoCount>,
) {
    if !buttons.contains(event.entity) {
        return;
    }
    for mut count in &mut count {
        count.activations += 1;
        info!(activations = count.activations, "长按确认激活");
    }
}
fn underlay(_: On<Activate>, mut count: Query<&mut HoldDemoCount>) {
    for mut count in &mut count {
        count.underlay += 1;
        info!(activations = count.underlay, "透明角落下层按钮激活");
    }
}
fn count_text(count: Query<&HoldDemoCount>, mut texts: Query<&mut Text, With<CountText>>) {
    for count in &count {
        for mut text in &mut texts {
            text.set_if_neq(Text::new(format!(
                "Activate = {} / Underlay = {}",
                count.activations, count.underlay
            )));
        }
    }
}
fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    if let Ok(action) = actions.get(event.entity) {
        let action = *action;
        commands.queue(move |world: &mut World| apply(world, action));
    }
}
fn shortcuts(world: &mut World) {
    let Some(keys) = world.get_resource::<ButtonInput<KeyCode>>() else {
        return;
    };
    let actions: Vec<_> = [
        (KeyCode::F7, Action::Disabled),
        (KeyCode::F8, Action::ParentDisabled),
        (KeyCode::F9, Action::Visibility),
    ]
    .into_iter()
    .filter_map(|(key, action)| keys.just_pressed(key).then_some(action))
    .collect();
    for action in actions {
        apply(world, action);
    }
}
fn apply(world: &mut World, action: Action) {
    let Some(entity) = world
        .query_filtered::<Entity, With<MainHold>>()
        .iter(world)
        .next()
    else {
        return;
    };
    match action {
        Action::Focus => world
            .resource_mut::<InputFocus>()
            .set(entity, FocusCause::Navigated),
        Action::Blur => world.resource_mut::<InputFocus>().clear(),
        Action::Disabled | Action::ParentDisabled => {
            let target = if matches!(action, Action::ParentDisabled) {
                let Some(parent) = world
                    .query_filtered::<Entity, With<HoldParent>>()
                    .iter(world)
                    .next()
                else {
                    return;
                };
                parent
            } else {
                entity
            };
            if world.get::<InteractionDisabled>(target).is_some() {
                world.entity_mut(target).remove::<InteractionDisabled>();
            } else {
                world.entity_mut(target).insert(InteractionDisabled);
            }
        }
        Action::Visibility => {
            let hidden = world.get::<Visibility>(entity) == Some(&Visibility::Hidden);
            world.entity_mut(entity).insert(if hidden {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
    if !matches!(action, Action::Focus | Action::Blur) {
        info!(?action, "切换长按按钮演示状态");
    }
}
