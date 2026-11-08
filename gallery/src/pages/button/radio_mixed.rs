const ITEM_GAP: f32 = 8.0;
const GROUP_GAP: f32 = 12.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 12.0;
const LABEL_FONT_SIZE: f32 = 18.0;
const TITLE_FONT_SIZE: f32 = 20.0;

use crate::{
    assets::GalleryImage,
    i18n::{ButtonKey, Language, TextKey, localized, localized_args, tr_args},
};
use bevy::{
    input_focus::InputFocus,
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton,
    radio_group::{WidgetryRadioGroupHeadless, WidgetryRadioOption, WidgetryRadioOptionHeadless},
    text::WidgetryText,
};

#[derive(SceneComponent, Clone, Reflect)]
#[reflect(Component)]
struct GalleryRadioGroup {
    user_changes: usize,
    next_card: usize,
}
impl Default for GalleryRadioGroup {
    fn default() -> Self {
        Self {
            user_changes: 0,
            next_card: 2,
        }
    }
}
#[derive(Component, Clone, Copy)]
enum ChoiceStyle {
    Card,
    Image,
}
#[derive(Component)]
struct Status;
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Add,
    Remove,
    Next,
    Disabled,
}

impl GalleryRadioGroup {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryRadioGroupHeadless
            Name("RadioMixedGroup")
            Node { width: percent(100), flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(CONTENT_GAP), row_gap: px(GROUP_GAP), align_items: AlignItems::Center, padding: UiRect::all(px(8)), border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(8)) }
            template(|_| Ok(BorderColor::all(Color::srgb_u8(90, 110, 130))))
            on(selected)
            Children [
                @WidgetryRadioOption Name("RadioMixedDefault") Node { min_width: px(140), height: px(80) }
                Children [@localized(TextKey::Button(ButtonKey::RadioMixedDefault)) WidgetryText]--
                @custom_option(ChoiceStyle::Card, 1, "RadioMixedCard".to_owned())--
                @custom_option(ChoiceStyle::Image, 0, "RadioMixedImage".to_owned())
            ]
        }
    }
}

fn custom_option(style: ChoiceStyle, number: usize, name: String) -> impl Scene {
    let image = matches!(style, ChoiceStyle::Image);
    bsn! {
        @WidgetryRadioOptionHeadless
        Name(name) template(move |_| Ok(style))
        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
        Node { width: px(if image { 220 } else { 190 }), height: px(if image { 112 } else { 80 }), flex_shrink: 0.0, border: UiRect::all(px(3)), border_radius: BorderRadius::all(px(12)), overflow: Overflow::clip(), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        BackgroundColor({Color::srgb_u8(25, 50, 70)})
        BorderColor
        @{image.then_some(bsn! {
            template(|context| Ok(ImageNode::new(context.entity.world().resource::<AssetServer>().load(GalleryImage::WindowBackground1.path()))))
        })}
        Children [
            @{(!image).then_some(bsn! {
                @localized_args(TextKey::Button(ButtonKey::RadioMixedCard), vec![("number", number.to_string())])
                Pickable::IGNORE TextFont { font_size: FontSize::Px(TITLE_FONT_SIZE) } TextColor({Color::WHITE})
            })}--
            @{image.then_some(bsn! {
                Pickable::IGNORE Node { position_type: PositionType::Absolute, left: px(0), right: px(0), bottom: px(0), height: px(32), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                BackgroundColor({Color::srgba(0.0, 0.0, 0.0, 0.7)})
                Children [@localized(TextKey::Button(ButtonKey::RadioMixedImage)) Pickable::IGNORE TextColor({Color::WHITE}) TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) }]
            })}
        ]
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("RadioMixedDemo")
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Button(ButtonKey::RadioMixedDescription)) WidgetryText--
            @GalleryRadioGroup--
            Name("RadioMixedStatus") template(|_| Ok(Status)) Text("") WidgetryText--
            Node { column_gap: px(CONTROL_GAP), row_gap: px(ITEM_GAP), flex_wrap: FlexWrap::Wrap }
            Children [
                @control(Action::Add, ButtonKey::RadioMixedAdd, "RadioMixedAdd")--
                @control(Action::Remove, ButtonKey::RadioMixedRemove, "RadioMixedRemove")--
                @control(Action::Next, ButtonKey::RadioMixedNext, "RadioMixedNext")--
                @control(Action::Disabled, ButtonKey::RadioMixedDisabled, "RadioMixedDisabled")
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
    app.register_type::<GalleryRadioGroup>().add_systems(
        PostUpdate,
        update_visuals
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::Button)),
    );
}

fn selected(event: On<ValueChange<usize>>, mut groups: Query<&mut GalleryRadioGroup>) {
    if let Ok(mut group) = groups.get_mut(event.source) {
        group.user_changes += 1;
        info!(entity=?event.source, index=event.value, user_changes=group.user_changes, "混合单选组用户选择已提交");
    }
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    groups: Query<Entity, With<GalleryRadioGroup>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    for root in &groups {
        commands.queue(move |world: &mut World| -> Result {
            match action {
                Action::Add => {
                    let mut group = world.get_mut::<GalleryRadioGroup>(root).ok_or_else(|| crate::diagnostics::error("混合单选组演示状态缺失"))?;
                    let number = group.next_card;
                    group.next_card += 1;
                    bevy_widgetry::scene::spawn_scene(world, bsn! {
                        @custom_option(ChoiceStyle::Card, number, format!("RadioMixedAdded{number}"))
                        template(move |_| Ok(ChildOf(root)))
                    }).map_err(|error| error.into_bevy_error())?;
                }
                Action::Remove | Action::Next => {
                    let members = world.get::<Children>(root).ok_or_else(|| crate::diagnostics::error("混合单选组成员缺失"))?.to_vec();
                    let selected = members.iter().position(|member| world.get::<Checked>(*member).is_some()).ok_or_else(|| crate::diagnostics::error("混合单选组当前选择缺失"))?;
                    if matches!(action, Action::Next) {
                        WidgetryRadioGroupHeadless::set_selected(&mut world.commands(), root, (selected + 1) % members.len());
                    } else if members.len() > 1 {
                        world.despawn(members[selected]);
                    } else {
                        info!(?root, "混合单选组保留最后一个选项，本次未删除");
                        return Ok(());
                    }
                }
                Action::Disabled => {
                    if world.get::<InteractionDisabled>(root).is_some() { world.entity_mut(root).remove::<InteractionDisabled>(); }
                    else { world.entity_mut(root).insert(InteractionDisabled); }
                }
            }
            info!(?root, ?action, "操作混合单选组演示");
            Ok(())
        });
    }
}

type LastStatus = Option<(Entity, Language, Option<usize>, usize, usize)>;
fn update_visuals(
    language: Res<Language>,
    focus: Res<InputFocus>,
    mut groups: Query<(Entity, &GalleryRadioGroup, &Children, &mut BorderColor)>,
    checked: Query<(), With<Checked>>,
    mut choices: Query<
        (
            Has<Checked>,
            Has<InteractionDisabled>,
            &Hovered,
            &mut BorderColor,
            &mut BackgroundColor,
            Option<&mut ImageNode>,
        ),
        (With<ChoiceStyle>, Without<GalleryRadioGroup>),
    >,
    mut labels: Query<&mut Text, With<Status>>,
    mut last: Local<LastStatus>,
) {
    for (root, group, members, mut border) in &mut groups {
        border.set_if_neq(BorderColor::all(if focus.get() == Some(root) {
            Color::srgb_u8(55, 205, 175)
        } else {
            Color::srgb_u8(90, 110, 130)
        }));
        let selected = members.iter().position(|member| checked.contains(member));
        let current = Some((root, *language, selected, members.len(), group.user_changes));
        if *last != current {
            *last = current;
            let value = tr_args(
                *language,
                TextKey::Button(ButtonKey::RadioMixedStatus),
                &[
                    (
                        "index",
                        selected.map_or_else(|| "-".to_owned(), |i| i.to_string()),
                    ),
                    ("count", members.len().to_string()),
                    ("changes", group.user_changes.to_string()),
                ],
            );
            for mut label in &mut labels {
                label.set_if_neq(Text(value.clone()));
            }
        }
    }
    for (selected, disabled, hovered, mut border, mut background, image) in &mut choices {
        let edge = if disabled {
            Color::srgb_u8(100, 110, 120)
        } else if selected {
            Color::srgb_u8(55, 205, 175)
        } else if hovered.0 {
            Color::srgb_u8(140, 210, 235)
        } else {
            Color::srgb_u8(70, 100, 125)
        };
        border.set_if_neq(BorderColor::all(edge));
        background.set_if_neq(BackgroundColor(if selected {
            Color::srgb_u8(20, 95, 85)
        } else {
            Color::srgb_u8(25, 50, 70)
        }));
        if let Some(mut image) = image {
            let color = Color::srgba(1.0, 1.0, 1.0, if disabled { 0.45 } else { 1.0 });
            if image.color != color {
                image.color = color;
            }
        }
    }
}
