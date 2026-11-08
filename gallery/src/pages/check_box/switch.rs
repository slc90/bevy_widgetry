const CHECKED_THUMB_LEFT: f32 = 44.0;
const ROW_MIN_HEIGHT: f32 = 52.0;
const TRACK_WIDTH: f32 = 76.0;
const TRACK_HEIGHT: f32 = 36.0;
const THUMB_DIAMETER: f32 = 28.0;
const THUMB_INSET: f32 = 4.0;

const ITEM_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 12.0;

use crate::i18n::{CheckBoxKey, Language, TextKey, localized, tr, tr_args};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton, check_box::WidgetryCheckBoxHeadless, text::WidgetryText,
};

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryCheckSwitch {
    requests: usize,
}
#[derive(Component)]
struct Track;
#[derive(Component)]
struct Thumb;
#[derive(Component)]
struct Status;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Focus,
    Disabled,
    Set,
}

impl GalleryCheckSwitch {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryCheckBoxHeadless
            Name("GalleryCheckSwitch")
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { min_height: px(ROW_MIN_HEIGHT), column_gap: px(CONTENT_GAP), align_items: AlignItems::Center }
            on(requested)
            Children [
                Name("GalleryCheckSwitchTrack") template(|_| Ok(Track)) Pickable::IGNORE
                Node { width: px(TRACK_WIDTH), height: px(TRACK_HEIGHT), flex_shrink: 0.0, border_radius: BorderRadius::MAX }
                BackgroundColor
                Children [
                    Name("GalleryCheckSwitchThumb") template(|_| Ok(Thumb)) Pickable::IGNORE
                    Node { width: px(THUMB_DIAMETER), height: px(THUMB_DIAMETER), position_type: PositionType::Absolute, top: px(THUMB_INSET), left: px(THUMB_INSET), border_radius: BorderRadius::MAX }
                    BackgroundColor({Color::WHITE})
                ]--
                @localized(TextKey::CheckBox(CheckBoxKey::ReceiveUpdates)) WidgetryText Pickable::IGNORE
            ]
        }
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("GalleryCheckSwitchDemo")
        Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::CheckBox(CheckBoxKey::SwitchDescription)) WidgetryText--
            @GalleryCheckSwitch--
            Name("GalleryCheckSwitchStatus") template(|_| Ok(Status)) Text("") WidgetryText--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(CONTROL_GAP), row_gap: px(ITEM_GAP) }
            Children [
                @control(Action::Focus, CheckBoxKey::SwitchFocus, "GalleryCheckSwitchFocus")--
                @control(Action::Disabled, CheckBoxKey::SwitchDisabled, "GalleryCheckSwitchDisabled")--
                @control(Action::Set, CheckBoxKey::SwitchSet, "GalleryCheckSwitchSet")
            ]
        ]
    }
}

fn control(action: Action, key: CheckBoxKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::CheckBox(key)) WidgetryText }) as Box<dyn SceneList>)} }
        Name(name) template(move |_| Ok(action)) on(operate)
    }
}

pub(super) fn install(app: &mut App) {
    app.register_type::<GalleryCheckSwitch>().add_systems(
        PostUpdate,
        update_visuals
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::CheckBox)),
    );
}

fn requested(event: On<ValueChange<bool>>, mut switches: Query<&mut GalleryCheckSwitch>) {
    if let Ok(mut switch) = switches.get_mut(event.source) {
        switch.requests += 1;
        info!(entity=?event.source, requested=event.value, requests=switch.requests, "请求切换二态开关");
    }
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    switches: Query<Entity, With<GalleryCheckSwitch>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    for root in &switches {
        commands.queue(move |world: &mut World| {
            match action {
                Action::Focus => world
                    .resource_mut::<InputFocus>()
                    .set(root, FocusCause::Navigated),
                Action::Disabled => {
                    if world.get::<InteractionDisabled>(root).is_some() {
                        world.entity_mut(root).remove::<InteractionDisabled>();
                    } else {
                        world.entity_mut(root).insert(InteractionDisabled);
                    }
                }
                Action::Set => {
                    if world.get::<Checked>(root).is_some() {
                        world.entity_mut(root).remove::<Checked>();
                    } else {
                        world.entity_mut(root).insert(Checked);
                    }
                }
            }
            info!(
                ?root,
                ?action,
                checked = world.get::<Checked>(root).is_some(),
                "操作二态开关演示"
            );
        });
    }
}

type LastStatus = Option<(Entity, Language, bool, usize)>;
fn update_visuals(
    language: Res<Language>,
    switches: Query<(
        Entity,
        &GalleryCheckSwitch,
        Has<Checked>,
        Has<InteractionDisabled>,
        &Hovered,
    )>,
    mut tracks: Query<(Entity, &ChildOf, &mut BackgroundColor), With<Track>>,
    mut thumbs: Query<(&ChildOf, &mut Node), With<Thumb>>,
    mut labels: Query<&mut Text, With<Status>>,
    mut last: Local<LastStatus>,
) {
    for (root, switch, checked, disabled, hovered) in &switches {
        let color = if disabled {
            Color::srgb_u8(105, 110, 120)
        } else if checked {
            Color::srgb_u8(20, 155, 120)
        } else if hovered.0 {
            Color::srgb_u8(90, 130, 160)
        } else {
            Color::srgb_u8(65, 80, 100)
        };
        for (track, parent, mut background) in &mut tracks {
            if parent.parent() != root {
                continue;
            }
            background.set_if_neq(BackgroundColor(color));
            for (parent, mut node) in &mut thumbs {
                if parent.parent() != track {
                    continue;
                }
                let left = px(if checked {
                    CHECKED_THUMB_LEFT
                } else {
                    THUMB_INSET
                });
                if node.left != left {
                    node.left = left;
                }
            }
        }
        let current = Some((root, *language, checked, switch.requests));
        if *last == current {
            continue;
        }
        *last = current;
        let value = tr_args(
            *language,
            TextKey::CheckBox(CheckBoxKey::SwitchStatus),
            &[
                (
                    "state",
                    tr(
                        *language,
                        TextKey::CheckBox(if checked {
                            CheckBoxKey::Checked
                        } else {
                            CheckBoxKey::Unchecked
                        }),
                    )
                    .to_owned(),
                ),
                ("count", switch.requests.to_string()),
            ],
        );
        for mut label in &mut labels {
            label.set_if_neq(Text(value.clone()));
        }
    }
}
