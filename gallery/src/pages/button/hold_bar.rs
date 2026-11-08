const BAR_WIDTH: f32 = 300.0;
const BAR_HEIGHT: f32 = 60.0;
const BAR_BORDER_WIDTH: f32 = 2.0;

const ITEM_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 8.0;
const LABEL_FONT_SIZE: f32 = 20.0;

use crate::i18n::{ButtonKey, Language, TextKey, localized, tr_args};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::WidgetryButton,
    hold_button::{
        WidgetryHoldButtonHeadless, WidgetryHoldButtonProgress, WidgetryHoldButtonState,
    },
    text::WidgetryText,
};

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryHoldBar {
    activations: usize,
}
#[derive(Component)]
struct Fill;
#[derive(Component)]
struct Status;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Focus,
    Disabled,
}

impl GalleryHoldBar {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryHoldButtonHeadless
            Name("GalleryHoldBar")
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { width: px(BAR_WIDTH), height: px(BAR_HEIGHT), border: UiRect::all(px(BAR_BORDER_WIDTH)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            BackgroundColor({Color::srgb_u8(21, 44, 64)})
            template(|_| Ok(BorderColor::all(Color::srgb_u8(100, 180, 210))))
            on(activated)
            Children [
                Name("GalleryHoldBarFill") template(|_| Ok(Fill)) Pickable::IGNORE
                Node { position_type: PositionType::Absolute, left: px(0), top: px(0), bottom: px(0), width: percent(0) }
                BackgroundColor({Color::srgb_u8(30, 110, 125)})--
                @localized(TextKey::Button(ButtonKey::HoldBarAction)) Pickable::IGNORE
                TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) } TextColor({Color::WHITE})
            ]
        }
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("GalleryHoldBarDemo")
        Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Button(ButtonKey::HoldBarDescription)) WidgetryText--
            @GalleryHoldBar--
            Name("GalleryHoldBarStatus") template(|_| Ok(Status)) Text("") WidgetryText--
            Node { column_gap: px(CONTROL_GAP), row_gap: px(ITEM_GAP), flex_wrap: FlexWrap::Wrap }
            Children [
                @control(Action::Focus, ButtonKey::HoldBarFocus, "GalleryHoldBarFocus")--
                @control(Action::Disabled, ButtonKey::HoldBarDisable, "GalleryHoldBarDisable")
            ]
        ]
    }
}

fn control(action: Action, key: ButtonKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(key)) WidgetryText }) as Box<dyn SceneList>)} }
        Name(name) template(move |_| Ok(action)) on(operate)
    }
}

pub(super) fn install(app: &mut App) {
    app.register_type::<GalleryHoldBar>()
        .add_systems(
            Update,
            shortcuts.run_if(in_state(crate::gallery::GalleryPage::Button)),
        )
        .add_systems(
            PostUpdate,
            update_visuals
                .after(bevy_widgetry::hold_button::WidgetryHoldButtonSystems::Update)
                .before(bevy::text::detect_text_needs_rerender)
                .before(bevy::ui::UiSystems::Content)
                .run_if(in_state(crate::gallery::GalleryPage::Button)),
        );
}

fn activated(event: On<Activate>, mut bars: Query<&mut GalleryHoldBar>) {
    if let Ok(mut bar) = bars.get_mut(event.entity) {
        bar.activations += 1;
        info!(entity=?event.entity, activations=bar.activations, "条形长按确认激活");
    }
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    if let Ok(action) = actions.get(event.entity) {
        let action = *action;
        commands.queue(move |world: &mut World| apply(world, action));
    }
}

fn shortcuts(keys: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keys.just_pressed(KeyCode::F6) {
        commands.queue(|world: &mut World| apply(world, Action::Disabled));
    }
}

fn apply(world: &mut World, action: Action) {
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<GalleryHoldBar>>()
        .iter(world)
        .collect();
    for entity in entities {
        match action {
            Action::Focus => world
                .resource_mut::<InputFocus>()
                .set(entity, FocusCause::Navigated),
            Action::Disabled => {
                if world.get::<InteractionDisabled>(entity).is_some() {
                    world.entity_mut(entity).remove::<InteractionDisabled>();
                } else {
                    world.entity_mut(entity).insert(InteractionDisabled);
                }
            }
        }
        info!(?entity, ?action, "切换条形长按确认演示状态");
    }
}

fn update_visuals(
    language: Res<Language>,
    mut bars: Query<(
        Ref<GalleryHoldBar>,
        Ref<WidgetryHoldButtonProgress>,
        &WidgetryHoldButtonState,
        &Hovered,
        Has<InteractionDisabled>,
        &mut BorderColor,
    )>,
    mut fills: Query<&mut Node, With<Fill>>,
    mut labels: Query<&mut Text, With<Status>>,
) {
    for (bar, progress, state, hovered, disabled, mut border) in &mut bars {
        let color = if disabled {
            Color::srgb_u8(90, 95, 105)
        } else if state.has_succeeded() {
            Color::srgb_u8(120, 255, 165)
        } else if state.is_holding() {
            Color::srgb_u8(255, 215, 115)
        } else if hovered.0 {
            Color::srgb_u8(180, 235, 250)
        } else {
            Color::srgb_u8(100, 180, 210)
        };
        border.set_if_neq(BorderColor::all(color));
        if progress.is_changed() {
            for mut fill in &mut fills {
                let width = percent(progress.value() * 100.0);
                if fill.width != width {
                    fill.width = width;
                }
            }
        }
        if !progress.is_changed() && !bar.is_changed() && !language.is_changed() {
            continue;
        }
        let text = tr_args(
            *language,
            TextKey::Button(ButtonKey::HoldBarStatus),
            &[
                ("progress", format!("{:.0}", progress.value() * 100.0)),
                ("count", bar.activations.to_string()),
            ],
        );
        for mut label in &mut labels {
            label.set_if_neq(Text(text.clone()));
        }
    }
}
