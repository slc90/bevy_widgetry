use crate::{
    gallery::GalleryPage,
    i18n::{ButtonKey, Language, TextKey, localized, tr, tr_args},
};
use bevy::{
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonHeadless, WidgetryButtonLoading},
    disabled::WidgetryEffectiveDisabled,
    text::WidgetryText,
};

#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component)]
struct GalleryButton {
    activations: usize,
}

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
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node {
                width: px(248), height: px(68), border: UiRect::all(px(2)),
                border_radius: BorderRadius { top_left: px(24), bottom_left: px(6), top_right: px(6), bottom_right: px(24) },
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            }
            BackgroundColor({Color::srgb_u8(15, 58, 72)})
            template(|_| Ok(BorderColor::all(Color::srgb_u8(62, 211, 190))))
            on(activated)
            Children [
                Pickable::IGNORE Node { position_type: PositionType::Absolute, left: px(10), top: px(16), bottom: px(16), width: px(4), border_radius: BorderRadius::all(px(2)) }
                BackgroundColor({Color::srgb_u8(62, 211, 190)})--
                Name("GalleryButtonLabel") template(|_| Ok(Label)) Pickable::IGNORE
                @localized(TextKey::Button(ButtonKey::CustomAction))
                TextColor({Color::srgb_u8(225, 255, 249)})
                TextFont { font_size: FontSize::Px(20.0) }--
                Name("GalleryButtonBusy") template(|_| Ok(Busy)) Pickable::IGNORE Visibility::Hidden
                Node { position_type: PositionType::Absolute, left: px(24), right: px(24), top: px(8), bottom: px(8), flex_direction: FlexDirection::Column, row_gap: px(5), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                Children [
                    Pickable::IGNORE @localized(TextKey::Button(ButtonKey::CustomWorking))
                    TextColor({Color::srgb_u8(225, 255, 249)}) TextFont { font_size: FontSize::Px(16.0) }--
                    Pickable::IGNORE Node { column_gap: px(6) } Children [@dot(0)-- @dot(1)-- @dot(2)]
                ]
            ]
        }
    }
}

fn dot(index: usize) -> impl Scene {
    bsn! {
        template(move |_| Ok(Dot(index))) Pickable::IGNORE
        Node { width: px(8), height: px(8), border_radius: BorderRadius::all(px(2)) }
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
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(10), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Button(ButtonKey::CustomTitle)) WidgetryText--
            @localized(TextKey::Button(ButtonKey::CustomDescription)) WidgetryText TextFont { font_size: FontSize::Px(14.0) }--
            Name("GalleryButtonParent") template(|_| Ok(DemoRequests::default())) Node
                Children [ @GalleryButton Name("GalleryButton") ]--
            Name("GalleryButtonStatus") template(|_| Ok(Status)) Text("") WidgetryText TextFont { font_size: FontSize::Px(14.0) }--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8) }
            Children [
                @control(Action::LocalDisabled, ButtonKey::CustomLocalDisabled, "GalleryButtonToggleLocalDisabled")--
                @control(Action::AncestorDisabled, ButtonKey::CustomAncestorDisabled, "GalleryButtonToggleAncestorDisabled")--
                @control(Action::Loading, ButtonKey::CustomLoading, "GalleryButtonToggleLoading")
            ]
        ]
    }
}

pub(super) fn install(app: &mut App) {
    app.register_type::<GalleryButton>().add_systems(
        PostUpdate,
        update_visuals
            .before(bevy::ui::UiSystems::Prepare)
            .run_if(in_state(GalleryPage::Button)),
    );
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
    &'static WidgetryButtonLoading,
    &'static WidgetryEffectiveDisabled,
    &'static Hovered,
    Has<Pressed>,
    &'static mut BackgroundColor,
    &'static mut BorderColor,
);

fn update_visuals(
    time: Res<Time>,
    language: Res<Language>,
    mut roots: Query<RootData, Without<Dot>>,
    mut labels: Query<(&ChildOf, &mut Visibility, &mut TextColor), With<Label>>,
    mut busy: Query<(&ChildOf, &mut Visibility), (With<Busy>, Without<Label>)>,
    ancestry: Query<&ChildOf>,
    mut dots: Query<(&ChildOf, &Dot, &mut UiTransform), Without<GalleryButton>>,
    mut status: Query<&mut Text, With<Status>>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    for (root, button, loading, disabled, hovered, pressed, mut background, mut border) in
        &mut roots
    {
        let active = loading.is_active();
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
        for (parent, dot, mut transform) in &mut dots {
            if ancestry
                .get(parent.parent())
                .and_then(|parent| ancestry.get(parent.parent()))
                .is_ok_and(|parent| parent.parent() == root)
            {
                let scale = if active {
                    0.7 + 0.3 * (time.elapsed_secs() * 7.0 - dot.0 as f32 * 1.2).sin()
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
