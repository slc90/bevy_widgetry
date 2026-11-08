const TAG_HEIGHT: f32 = 52.0;
const TAG_MIN_WIDTH: f32 = 220.0;
const TAG_PADDING_X: f32 = 12.0;
const TAG_PADDING_Y: f32 = 6.0;
const TAG_UNDERLINE_WIDTH: f32 = 3.0;

const ITEM_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 10.0;
const LABEL_FONT_SIZE: f32 = 20.0;
const TITLE_FONT_SIZE: f32 = 28.0;

use crate::i18n::{ButtonKey, Language, TextKey, localized, tr, tr_args};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::WidgetryButton,
    text::WidgetryText,
    toggle_button::{WidgetryToggleButtonChanged, WidgetryToggleButtonHeadless},
};

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryToggleTag {
    changes: usize,
    local_disabled: bool,
    parent_disabled: bool,
}
#[derive(Component)]
struct Parent;
#[derive(Component)]
struct Icon;
#[derive(Component)]
struct Label;
#[derive(Component)]
struct Status;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Set,
    Focus,
    LocalDisabled,
    ParentDisabled,
}

impl GalleryToggleTag {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryToggleButtonHeadless
            Name("GalleryToggleTag")
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { height: px(TAG_HEIGHT), min_width: px(TAG_MIN_WIDTH), padding: UiRect::axes(px(TAG_PADDING_X), px(TAG_PADDING_Y)), border: UiRect::bottom(px(TAG_UNDERLINE_WIDTH)), column_gap: px(CONTENT_GAP), align_items: AlignItems::Center }
            template(|_| Ok(BorderColor::all(Color::srgb_u8(90, 130, 160))))
            on(changed)
            Children [
                template(|_| Ok(Icon)) Pickable::IGNORE Text("☆") TextFont { font_size: FontSize::Px(TITLE_FONT_SIZE) }--
                template(|_| Ok(Label)) Pickable::IGNORE Text("") TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) }
            ]
        }
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Button(ButtonKey::ToggleTagDescription)) WidgetryText--
            Name("GalleryToggleTagParent") template(|_| Ok(Parent)) Node Children [@GalleryToggleTag]--
            Name("GalleryToggleTagStatus") template(|_| Ok(Status)) Text("") WidgetryText--
            Node { column_gap: px(CONTROL_GAP), row_gap: px(ITEM_GAP), flex_wrap: FlexWrap::Wrap }
            Children [
                @control(Action::Set, ButtonKey::ToggleProgrammatic, "GalleryToggleTagSet")--
                @control(Action::Focus, ButtonKey::ToggleTagFocus, "GalleryToggleTagFocus")--
                @control(Action::LocalDisabled, ButtonKey::CustomLocalDisabled, "GalleryToggleTagDisable")--
                @control(Action::ParentDisabled, ButtonKey::CustomAncestorDisabled, "GalleryToggleTagParentDisable")
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
    app.register_type::<GalleryToggleTag>().add_systems(
        PostUpdate,
        (update_visuals, update_status)
            .chain()
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::Button)),
    );
}

fn changed(event: On<WidgetryToggleButtonChanged>, mut tags: Query<&mut GalleryToggleTag>) {
    if let Ok(mut tag) = tags.get_mut(event.entity) {
        tag.changes += 1;
        info!(entity=?event.entity, selected=event.selected, changes=tag.changes, "收藏标签状态已提交");
    }
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    tags: Query<Entity, With<GalleryToggleTag>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    for root in &tags {
        commands.queue(move |world: &mut World| -> Result {
            match action {
                Action::Set => {
                    let selected = WidgetryToggleButtonHeadless::selected(world, root)
                        .map_err(|e| e.into_bevy_error())?;
                    WidgetryToggleButtonHeadless::set_selected_in_world(world, root, !selected)
                        .map_err(|e| e.into_bevy_error())?;
                }
                Action::Focus => world
                    .resource_mut::<InputFocus>()
                    .set(root, FocusCause::Navigated),
                Action::LocalDisabled | Action::ParentDisabled => {
                    let target = if matches!(action, Action::ParentDisabled) {
                        world
                            .query_filtered::<Entity, With<Parent>>()
                            .single(world)
                            .map_err(crate::diagnostics::error)?
                    } else {
                        root
                    };
                    let mut tag = world
                        .get_mut::<GalleryToggleTag>(root)
                        .ok_or_else(|| crate::diagnostics::error("收藏标签演示状态缺失"))?;
                    let request = if matches!(action, Action::LocalDisabled) {
                        &mut tag.local_disabled
                    } else {
                        &mut tag.parent_disabled
                    };
                    *request = !*request;
                    if *request {
                        world.entity_mut(target).insert(InteractionDisabled);
                    } else {
                        world.entity_mut(target).remove::<InteractionDisabled>();
                    }
                }
            }
            info!(?root, ?action, "操作收藏标签演示");
            Ok(())
        });
    }
}

fn update_visuals(
    language: Res<Language>,
    mut tags: Query<
        (
            Entity,
            Has<Checked>,
            Has<InteractionDisabled>,
            &Hovered,
            &mut BorderColor,
        ),
        With<GalleryToggleTag>,
    >,
    mut texts: Query<
        (&ChildOf, &mut Text, &mut TextColor, Has<Icon>),
        Or<(With<Icon>, With<Label>)>,
    >,
) {
    for (root, selected, disabled, hovered, mut border) in &mut tags {
        let color = if disabled {
            Color::srgb_u8(110, 115, 125)
        } else if selected {
            Color::srgb_u8(55, 205, 175)
        } else if hovered.0 {
            Color::srgb_u8(120, 195, 225)
        } else {
            Color::srgb_u8(90, 130, 160)
        };
        border.set_if_neq(BorderColor::all(color));
        for (parent, mut text, mut foreground, icon) in &mut texts {
            if parent.parent() != root {
                continue;
            }
            foreground.set_if_neq(TextColor(color));
            let value = if icon {
                if selected { "★" } else { "☆" }
            } else {
                tr(
                    *language,
                    TextKey::Button(if selected {
                        ButtonKey::ToggleTagOn
                    } else {
                        ButtonKey::ToggleTagOff
                    }),
                )
            };
            if text.0 != value {
                text.0 = value.to_owned();
            }
        }
    }
}

type LastStatus = Option<(Entity, Language, bool, usize)>;
fn update_status(world: &mut World, mut last: Local<LastStatus>) -> Result {
    let language = *world.resource::<Language>();
    let entries: Vec<_> = world
        .query::<(Entity, &GalleryToggleTag)>()
        .iter(world)
        .map(|(root, tag)| (root, tag.changes))
        .collect();
    for (root, count) in entries {
        let selected =
            WidgetryToggleButtonHeadless::selected(world, root).map_err(|e| e.into_bevy_error())?;
        let current = Some((root, language, selected, count));
        if *last == current {
            continue;
        }
        *last = current;
        let value = tr_args(
            language,
            TextKey::Button(ButtonKey::ToggleTagStatus),
            &[
                (
                    "state",
                    tr(
                        language,
                        TextKey::Button(if selected {
                            ButtonKey::ToggleTagOn
                        } else {
                            ButtonKey::ToggleTagOff
                        }),
                    )
                    .to_owned(),
                ),
                ("count", count.to_string()),
            ],
        );
        for mut text in world
            .query_filtered::<&mut Text, With<Status>>()
            .iter_mut(world)
        {
            text.set_if_neq(Text(value.clone()));
        }
    }
    Ok(())
}
