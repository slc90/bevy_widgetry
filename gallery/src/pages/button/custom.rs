const DOT_BASE_SCALE: f32 = 0.7;
const DOT_SCALE_AMPLITUDE: f32 = 0.3;
const DOT_ANGULAR_SPEED: f32 = 7.0;
const DOT_PHASE_OFFSET: f32 = 1.2;

const BUTTON_WIDTH: f32 = 248.0;
const BUTTON_HEIGHT: f32 = 68.0;
const BUTTON_BORDER_WIDTH: f32 = 2.0;
const LARGE_CORNER_RADIUS: f32 = 24.0;
const SMALL_CORNER_RADIUS: f32 = 6.0;
const STRIPE_LEFT: f32 = 10.0;
const STRIPE_VERTICAL_INSET: f32 = 16.0;
const STRIPE_WIDTH: f32 = 4.0;
const BUSY_HORIZONTAL_INSET: f32 = 24.0;
const BUSY_VERTICAL_INSET: f32 = 8.0;
const STAR_TOP: f32 = 2.0;
const STAR_RIGHT: f32 = 8.0;
const ORNAMENT_CORNER_RADIUS: f32 = 2.0;
const DOT_SIZE: f32 = 8.0;

const ITEM_GAP: f32 = 5.0;
const GROUP_GAP: f32 = 8.0;
const SECTION_GAP: f32 = 10.0;
const CONTROL_GAP: f32 = 6.0;
const CONTENT_GAP: f32 = 8.0;
const LABEL_FONT_SIZE: f32 = 14.0;
const CONTENT_FONT_SIZE: f32 = 16.0;
const SUBHEADING_FONT_SIZE: f32 = 18.0;
const TITLE_FONT_SIZE: f32 = 20.0;

use crate::{
    gallery::GalleryPage,
    i18n::{ButtonKey, Language, TextKey, localized, tr, tr_args},
};
use bevy::{
    picking::{hover::Hovered, pointer::PointerId},
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonHeadless, WidgetryButtonLoading},
    cursor::WidgetryCursorLoading,
    disabled::WidgetryEffectiveDisabled,
    text::WidgetryText,
};

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryButton {
    activations: usize,
    mark_changes: usize,
}

#[derive(Component, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryButtonMarked(bool);

#[derive(EntityEvent)]
struct GalleryButtonMarkedChanged {
    entity: Entity,
    marked: bool,
}

#[derive(Component, Default)]
struct SecondaryGesture(Option<SecondaryPress>);

struct SecondaryPress {
    pointer: PointerId,
    target: bevy::camera::NormalizedRenderTarget,
    dragged: bool,
}

#[derive(Component)]
struct Star;

#[derive(Component, Default)]
struct DemoRequests {
    local: bool,
    ancestor: bool,
}

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    LocalDisabled,
    AncestorDisabled,
    Loading,
}

#[derive(Component)]
struct Label;
#[derive(Component)]
struct Busy;
#[derive(Component)]
struct Dot(usize);
#[derive(Component)]
struct Status;

impl GalleryButton {
    fn scene() -> impl Scene {
        bsn! {
            @WidgetryButtonHeadless
            WidgetryButtonLoading::default()
            GalleryButtonMarked::default()
            template(|_| Ok(SecondaryGesture::default()))
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node {
                width: px(BUTTON_WIDTH), height: px(BUTTON_HEIGHT), border: UiRect::all(px(BUTTON_BORDER_WIDTH)),
                border_radius: BorderRadius { top_left: px(LARGE_CORNER_RADIUS), bottom_left: px(SMALL_CORNER_RADIUS), top_right: px(SMALL_CORNER_RADIUS), bottom_right: px(LARGE_CORNER_RADIUS) },
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            }
            BackgroundColor({Color::srgb_u8(15, 58, 72)})
            template(|_| Ok(BorderColor::all(Color::srgb_u8(62, 211, 190))))
            on(activated)
            on(secondary_press)
            on(secondary_drag)
            on(secondary_cancel)
            on(secondary_click)
            on(disabled_gesture)
            on(marked_changed)
            Children [
                Pickable::IGNORE Node { position_type: PositionType::Absolute, left: px(STRIPE_LEFT), top: px(STRIPE_VERTICAL_INSET), bottom: px(STRIPE_VERTICAL_INSET), width: px(STRIPE_WIDTH), border_radius: BorderRadius::all(px(ORNAMENT_CORNER_RADIUS)) }
                BackgroundColor({Color::srgb_u8(62, 211, 190)})--
                Name("GalleryButtonLabel") template(|_| Ok(Label)) Pickable::IGNORE
                @localized(TextKey::Button(ButtonKey::CustomAction))
                TextColor({Color::srgb_u8(225, 255, 249)})
                TextFont { font_size: FontSize::Px(TITLE_FONT_SIZE) }--
                Name("GalleryButtonBusy") template(|_| Ok(Busy)) Pickable::IGNORE Visibility::Hidden
                Node { position_type: PositionType::Absolute, left: px(BUSY_HORIZONTAL_INSET), right: px(BUSY_HORIZONTAL_INSET), top: px(BUSY_VERTICAL_INSET), bottom: px(BUSY_VERTICAL_INSET), flex_direction: FlexDirection::Column, row_gap: px(ITEM_GAP), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                Children [
                    Pickable::IGNORE @localized(TextKey::Button(ButtonKey::CustomWorking))
                    TextColor({Color::srgb_u8(225, 255, 249)}) TextFont { font_size: FontSize::Px(CONTENT_FONT_SIZE) }--
                    Pickable::IGNORE Node { column_gap: px(CONTROL_GAP) } Children [@dot(0)-- @dot(1)-- @dot(2)]
                ]--
                Name("GalleryButtonStar") template(|_| Ok(Star)) Pickable::IGNORE Visibility::Hidden
                Text("★") TextFont { font_size: FontSize::Px(SUBHEADING_FONT_SIZE) } TextColor({Color::srgb_u8(255, 218, 90)})
                Node { position_type: PositionType::Absolute, top: px(STAR_TOP), right: px(STAR_RIGHT) }
            ]
        }
    }
}

fn dot(index: usize) -> impl Scene {
    bsn! {
        template(move |_| Ok(Dot(index))) Pickable::IGNORE
        Node { width: px(DOT_SIZE), height: px(DOT_SIZE), border_radius: BorderRadius::all(px(ORNAMENT_CORNER_RADIUS)) }
        BackgroundColor({Color::srgb_u8(62, 211, 190)})
    }
}

fn control(action: Action, key: ButtonKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Button(key)) WidgetryText }) as Box<dyn SceneList>)} }
        template(move |_| Ok(action)) Name(name) on(operate)
    }
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("GalleryButtonDemo")
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(SECTION_GAP), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Button(ButtonKey::CustomTitle)) WidgetryText--
            @localized(TextKey::Button(ButtonKey::CustomDescription)) WidgetryText TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) }--
            @localized(TextKey::Button(ButtonKey::CustomMarkDescription)) WidgetryText TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) }--
            Name("GalleryButtonParent") template(|_| Ok(DemoRequests::default())) Node
                Children [ @GalleryButton Name("GalleryButton") ]--
            Name("GalleryButtonStatus") template(|_| Ok(Status)) Text("") WidgetryText TextFont { font_size: FontSize::Px(LABEL_FONT_SIZE) }--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(CONTENT_GAP), row_gap: px(GROUP_GAP) }
            Children [
                @control(Action::LocalDisabled, ButtonKey::CustomLocalDisabled, "GalleryButtonToggleLocalDisabled")--
                @control(Action::AncestorDisabled, ButtonKey::CustomAncestorDisabled, "GalleryButtonToggleAncestorDisabled")--
                @control(Action::Loading, ButtonKey::CustomLoading, "GalleryButtonToggleLoading")
            ]
        ]
    }
}

pub(super) fn install(app: &mut App) {
    app.register_type::<GalleryButton>()
        .register_type::<GalleryButtonMarked>()
        .add_systems(
            PostUpdate,
            update_visuals
                .before(bevy::ui::UiSystems::Prepare)
                .run_if(in_state(GalleryPage::Button)),
        );
}

fn secondary_press(
    event: On<PointerPress>,
    mut buttons: Query<(&WidgetryEffectiveDisabled, &mut SecondaryGesture), With<GalleryButton>>,
) {
    if event.button != PointerButton::Secondary {
        return;
    }
    if let Ok((disabled, mut gesture)) = buttons.get_mut(event.entity) {
        gesture.0 = (!disabled.is_disabled()).then(|| SecondaryPress {
            pointer: event.pointer.id,
            target: event.pointer.target.clone(),
            dragged: false,
        });
    }
}

fn secondary_drag(
    event: On<PointerDragStart>,
    mut gestures: Query<&mut SecondaryGesture, With<GalleryButton>>,
) {
    if event.button != PointerButton::Secondary {
        return;
    }
    if let Ok(mut gesture) = gestures.get_mut(event.entity)
        && let Some(press) = &mut gesture.0
        && press.pointer == event.pointer.id
        && press.target == event.pointer.target
    {
        press.dragged = true;
    }
}

fn secondary_cancel(
    event: On<PointerCancel>,
    mut gestures: Query<&mut SecondaryGesture, With<GalleryButton>>,
) {
    if let Ok(mut gesture) = gestures.get_mut(event.entity)
        && gesture.0.as_ref().is_some_and(|press| {
            press.pointer == event.pointer.id && press.target == event.pointer.target
        })
    {
        gesture.0 = None;
    }
}

fn disabled_gesture(
    event: On<Insert<WidgetryEffectiveDisabled>>,
    mut buttons: Query<(&WidgetryEffectiveDisabled, &mut SecondaryGesture), With<GalleryButton>>,
) {
    if let Ok((disabled, mut gesture)) = buttons.get_mut(event.entity)
        && disabled.is_disabled()
    {
        gesture.0 = None;
    }
}

fn secondary_click(
    event: On<PointerClick>,
    mut buttons: Query<
        (
            &WidgetryEffectiveDisabled,
            &mut SecondaryGesture,
            &mut GalleryButtonMarked,
        ),
        With<GalleryButton>,
    >,
    mut commands: Commands,
) {
    if event.button != PointerButton::Secondary {
        return;
    }
    let Ok((disabled, mut gesture, mut marked)) = buttons.get_mut(event.entity) else {
        return;
    };
    if !gesture.0.as_ref().is_some_and(|press| {
        press.pointer == event.pointer.id && press.target == event.pointer.target
    }) {
        return;
    }
    let Some(press) = gesture.0.take() else {
        return;
    };
    if disabled.is_disabled() || press.dragged {
        return;
    }
    marked.0 = !marked.0;
    commands.trigger(GalleryButtonMarkedChanged {
        entity: event.entity,
        marked: marked.0,
    });
}

fn marked_changed(
    event: On<GalleryButtonMarkedChanged>,
    mut buttons: Query<(&GalleryButtonMarked, &mut GalleryButton)>,
) {
    if let Ok((marked, mut button)) = buttons.get_mut(event.entity) {
        button.mark_changes += 1;
        info!(entity = ?event.entity, marked = marked.0, notified_marked = event.marked, changes = button.mark_changes, "切换 GalleryButton 标记");
    }
}

fn activated(event: On<Activate>, mut buttons: Query<&mut GalleryButton>) {
    if let Ok(mut button) = buttons.get_mut(event.entity) {
        button.activations += 1;
        info!(entity = ?event.entity, activations = button.activations, "激活自定义 GalleryButton");
    }
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    buttons: Query<Entity, With<GalleryButton>>,
    parents: Query<Entity, With<DemoRequests>>,
    mut commands: Commands,
) -> Result {
    let Ok(&action) = actions.get(event.entity) else {
        return Ok(());
    };
    let root = buttons.single().map_err(crate::diagnostics::error)?;
    let parent = parents.single().map_err(crate::diagnostics::error)?;
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<GalleryButton>(root).is_none() {
            return Ok(());
        }
        let enabled = if matches!(action, Action::Loading) {
            let active = !WidgetryButtonLoading::get(world, root)
                .map_err(|error| error.into_bevy_error())?
                .is_active();
            WidgetryButtonLoading::set_in_world(world, root, active)
                .map_err(|error| error.into_bevy_error())?;
            active
        } else {
            let mut requests = world
                .get_mut::<DemoRequests>(parent)
                .ok_or_else(|| crate::diagnostics::error("GalleryButton 演示请求状态缺失"))?;
            let requested = if matches!(action, Action::LocalDisabled) {
                &mut requests.local
            } else {
                &mut requests.ancestor
            };
            *requested = !*requested;
            let enabled = *requested;
            let target = if matches!(action, Action::LocalDisabled) {
                root
            } else {
                parent
            };
            if enabled {
                world.entity_mut(target).insert(InteractionDisabled);
            } else {
                world.entity_mut(target).remove::<InteractionDisabled>();
            }
            world.flush();
            enabled
        };
        info!(?root, ?action, enabled, "切换 GalleryButton 演示限制");
        Ok(())
    });
    Ok(())
}

type RootData = (
    Entity,
    &'static GalleryButton,
    Ref<'static, WidgetryButtonLoading>,
    &'static GalleryButtonMarked,
    &'static WidgetryEffectiveDisabled,
    &'static Hovered,
    Has<Pressed>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
);

fn update_visuals(
    time: Res<Time>,
    mut commands: Commands,
    language: Res<Language>,
    mut roots: Query<RootData, Without<Dot>>,
    mut labels: Query<(&ChildOf, &mut Visibility, &mut TextColor), With<Label>>,
    mut busy: Query<(&ChildOf, &mut Visibility), (With<Busy>, Without<Label>)>,
    mut stars: Query<(&ChildOf, &mut Visibility), (With<Star>, Without<Label>, Without<Busy>)>,
    ancestry: Query<&ChildOf>,
    mut dots: Query<(&ChildOf, &Dot, &mut UiTransform), Without<GalleryButton>>,
    mut status: Query<&mut Text, With<Status>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    for (root, button, loading, marked, disabled, hovered, pressed, mut background, mut border) in
        &mut roots
    {
        let active = loading.is_active();
        if loading.is_changed() {
            if active {
                commands.entity(root).try_insert(WidgetryCursorLoading);
            } else {
                commands.entity(root).try_remove::<WidgetryCursorLoading>();
            }
        }
        let state = if active {
            ButtonKey::CustomWorking
        } else if disabled.is_disabled() {
            ButtonKey::CustomDisabled
        } else if pressed {
            ButtonKey::CustomPressed
        } else if hovered.0 {
            ButtonKey::CustomHovered
        } else {
            ButtonKey::CustomNormal
        };
        let (fill, edge, foreground) = if active {
            (
                Color::srgb_u8(15, 58, 72),
                Color::srgb_u8(62, 211, 190),
                Color::srgb_u8(225, 255, 249),
            )
        } else if disabled.is_disabled() {
            (
                Color::srgb_u8(45, 50, 57),
                Color::srgb_u8(86, 95, 108),
                Color::srgb_u8(138, 147, 158),
            )
        } else if pressed {
            (
                Color::srgb_u8(112, 230, 206),
                Color::WHITE,
                Color::srgb_u8(6, 40, 52),
            )
        } else if hovered.0 {
            (
                Color::srgb_u8(23, 94, 105),
                Color::srgb_u8(160, 255, 226),
                Color::WHITE,
            )
        } else {
            (
                Color::srgb_u8(15, 58, 72),
                Color::srgb_u8(62, 211, 190),
                Color::srgb_u8(225, 255, 249),
            )
        };
        background.set_if_neq(BackgroundColor(fill));
        border.set_if_neq(BorderColor::all(edge));
        for (parent, mut visibility, mut color) in &mut labels {
            if parent.parent() == root {
                visibility.set_if_neq(if active {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                });
                color.set_if_neq(TextColor(foreground));
            }
        }
        for (parent, mut visibility) in &mut busy {
            if parent.parent() == root {
                visibility.set_if_neq(if active {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
        for (parent, mut visibility) in &mut stars {
            if parent.parent() == root {
                visibility.set_if_neq(if marked.0 {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
        }
        for (parent, dot, mut transform) in &mut dots {
            if ancestry
                .get(parent.parent())
                .and_then(|parent| ancestry.get(parent.parent()))
                .is_ok_and(|parent| parent.parent() == root)
            {
                let scale = if active {
                    DOT_BASE_SCALE
                        + DOT_SCALE_AMPLITUDE
                            * (time.elapsed_secs() * DOT_ANGULAR_SPEED
                                - dot.0 as f32 * DOT_PHASE_OFFSET)
                                .sin()
                } else {
                    1.0
                };
                transform.set_if_neq(UiTransform {
                    scale: Vec2::splat(scale),
                    ..default()
                });
            }
        }
        let text = tr_args(
            *language,
            TextKey::Button(ButtonKey::CustomStatus),
            &[
                ("count", button.activations.to_string()),
                ("state", tr(*language, TextKey::Button(state)).to_owned()),
                (
                    "marked",
                    tr(
                        *language,
                        TextKey::Button(if marked.0 {
                            ButtonKey::CustomMarked
                        } else {
                            ButtonKey::CustomUnmarked
                        }),
                    )
                    .to_owned(),
                ),
                ("changes", button.mark_changes.to_string()),
            ],
        );
        for mut output in &mut status {
            output.set_if_neq(Text::new(text.clone()));
        }
        if active {
            redraw.write(bevy::window::RequestRedraw);
        }
    }
}
