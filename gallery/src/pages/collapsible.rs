use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{CollapsibleKey, Language, LocalizedText, TextKey, localized};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use bevy_widgetry::{
    button::WidgetryButton,
    check_box::WidgetryCheckBox,
    collapsible::{WidgetryCollapsible, WidgetryCollapsibleChanged},
    slider::WidgetrySlider,
    text::WidgetryText,
    text_field::WidgetryTextField,
};

pub(crate) struct CollapsibleDemoPlugin;

impl Plugin for CollapsibleDemoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CollapsibleDemoState>()
            .add_systems(OnEnter(GalleryPage::Collapsible), enter)
            .add_systems(OnExit(GalleryPage::Collapsible), exit)
            .add_systems(Update, shortcuts.run_if(in_state(GalleryPage::Collapsible)))
            .add_systems(
                PostUpdate,
                refresh
                    .before(bevy::ui::UiSystems::Prepare)
                    .run_if(in_state(GalleryPage::Collapsible)),
            );
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
struct CollapsibleDemoState {
    events: u64,
    last: Option<bool>,
    root_disabled: bool,
}

#[derive(Component, FromTemplate)]
struct ExampleParts {
    card: Entity,
    status: Entity,
}

#[derive(Component, Clone, Copy)]
enum Action {
    Expand,
    Collapse,
    Same,
    RootDisabled,
    HeaderDisabled,
}

#[derive(Component, Clone, Copy)]
struct Shortcut(KeyCode);

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Collapsible, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Collapsible)
}

fn label(key: CollapsibleKey) -> impl Scene {
    bsn! { @localized(TextKey::Collapsible(key)) WidgetryText }
}

fn scene() -> impl Scene {
    bsn! {
        Name("CollapsibleDemo")
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16), flex_shrink: 0.0 }
        Children [
            @label(CollapsibleKey::Description) Node { max_width: percent(100) }--
            Node { width: percent(100), flex_wrap: FlexWrap::Wrap, column_gap: px(24), row_gap: px(20), align_items: AlignItems::Start }
            Children [
                @example("CollapsibleBasic", true, CollapsibleKey::Basic, CollapsibleKey::BasicHint, KeyCode::F8, bsn_list! {
                    @WidgetryCheckBox Name("CollapsibleCheckBox") Children [@label(CollapsibleKey::Enabled)]--
                    @label(CollapsibleKey::Volume)--
                    @WidgetrySlider { @value: {Some(35.0)} } Name("CollapsibleSlider") Node { width: percent(100) }
                })--
                @example("CollapsibleAdvanced", false, CollapsibleKey::Advanced, CollapsibleKey::AdvancedHint, KeyCode::F9, bsn_list! {
                    @label(CollapsibleKey::EditHint)--
                    @WidgetryTextField Name("CollapsibleTextField") Node { width: percent(100) }
                })
            ]
        ]
    }
}

fn example(
    name: &'static str,
    expanded: bool,
    title: CollapsibleKey,
    hint: CollapsibleKey,
    key: KeyCode,
    content: impl SceneList,
) -> impl Scene {
    let content = Some(Box::new(bsn_list! {
        Node { flex_direction: FlexDirection::Column, row_gap: px(10), width: percent(100) } Children [{content}]
    }) as Box<dyn SceneList>);
    bsn! {
        ExampleParts { card: #Card, status: #Status }
        Node { width: px(420), max_width: percent(100), min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(10), flex_shrink: 0.0 }
        Children [
            #Card Name(name)
            template(|_| Ok(CollapsibleDemoState::default()))
            template(move |_| Ok(Shortcut(key)))
            @WidgetryCollapsible {
                @expanded: expanded,
                @header: {Some(Box::new(bsn! {
                    Node { flex_direction: FlexDirection::Column, row_gap: px(4), min_width: px(0) }
                    Children [@label(title)-- @label(hint) TextFont { font_size: bevy::text::FontSize::Px(14.0) }]
                }) as Box<dyn Scene>)},
                @children: {content},
            }
            Node { width: percent(100) } on(changed)--
            Node { flex_wrap: FlexWrap::Wrap, column_gap: px(6), row_gap: px(6) }
            Children [
                @action(name, CollapsibleKey::Expand, Action::Expand)--
                @action(name, CollapsibleKey::Collapse, Action::Collapse)--
                @action(name, CollapsibleKey::Same, Action::Same)--
                @action(name, CollapsibleKey::RootDisabled, Action::RootDisabled)--
                @action(name, CollapsibleKey::HeaderDisabled, Action::HeaderDisabled)
            ]--
            #Status Name({format!("{name}Status")}) Text::new("") WidgetryText Node { max_width: percent(100) }
        ]
    }
}

fn action(prefix: &'static str, key: CollapsibleKey, action: Action) -> impl Scene {
    bsn! {
        Name({format!("{prefix}{key:?}")})
        template(move |_| Ok(action))
        @WidgetryButton { @children: bsn_list! { @label(key) } } on(activate)
    }
}

fn changed(event: On<WidgetryCollapsibleChanged>, mut states: Query<&mut CollapsibleDemoState>) {
    if let Ok(mut state) = states.get_mut(event.entity) {
        state.events += 1;
        state.last = Some(event.expanded);
        info!(entity = ?event.entity, expanded = event.expanded, "Collapsible 展开状态已变化");
    }
}

fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    parents: Query<&ChildOf>,
    examples: Query<&ExampleParts>,
    mut commands: Commands,
) -> Result {
    let action = *actions
        .get(event.entity)
        .map_err(|_| demo_error("Collapsible 操作类型缺失"))?;
    let root = parents
        .iter_ancestors(event.entity)
        .find_map(|parent| examples.get(parent).ok().map(|parts| parts.card))
        .ok_or_else(|| demo_error("Collapsible 操作实例缺失"))?;
    commands.queue(move |world: &mut World| perform(world, root, action));
    Ok(())
}

fn perform(world: &mut World, root: Entity, action: Action) -> Result {
    match action {
        Action::Expand | Action::Collapse | Action::Same => {
            let expanded = match action {
                Action::Expand => true,
                Action::Collapse => false,
                _ => WidgetryCollapsible::expanded(world, root)?,
            };
            let changed = WidgetryCollapsible::set_expanded_in_world(world, root, expanded)?;
            info!(?root, expanded, changed, "程序设置 Collapsible 展开状态");
        }
        Action::RootDisabled => {
            let disabled = !world
                .get::<CollapsibleDemoState>(root)
                .ok_or_else(|| demo_error("Collapsible 演示状态缺失"))?
                .root_disabled;
            if disabled {
                world.entity_mut(root).insert(InteractionDisabled);
            } else {
                world.entity_mut(root).remove::<InteractionDisabled>();
            }
            world
                .get_mut::<CollapsibleDemoState>(root)
                .ok_or_else(|| demo_error("Collapsible 演示状态缺失"))?
                .root_disabled = disabled;
            info!(?root, disabled, "切换 Collapsible 根禁用请求");
        }
        Action::HeaderDisabled => {
            let disabled = !WidgetryCollapsible::header_disabled(world, root)?;
            WidgetryCollapsible::set_header_disabled_in_world(world, root, disabled)?;
            info!(?root, disabled, "切换 Collapsible Header 局部禁用请求");
        }
    }
    Ok(())
}

fn shortcuts(
    input: Res<ButtonInput<KeyCode>>,
    roots: Query<(Entity, &Shortcut)>,
    mut commands: Commands,
) {
    for (root, shortcut) in &roots {
        if input.just_pressed(shortcut.0) {
            commands.queue(move |world: &mut World| {
                let expanded = WidgetryCollapsible::expanded(world, root)?;
                perform(
                    world,
                    root,
                    if expanded {
                        Action::Collapse
                    } else {
                        Action::Expand
                    },
                )
            });
        }
    }
}

fn refresh(world: &mut World) -> Result {
    let examples = world
        .query::<&ExampleParts>()
        .iter(world)
        .map(|parts| (parts.card, parts.status))
        .collect::<Vec<_>>();
    for (root, status) in examples {
        let state = world
            .get::<CollapsibleDemoState>(root)
            .ok_or_else(|| demo_error("Collapsible 演示状态缺失"))?;
        let binding = LocalizedText::with_args(
            TextKey::Collapsible(CollapsibleKey::Status),
            vec![
                (
                    "expanded",
                    WidgetryCollapsible::expanded(world, root)?.to_string(),
                ),
                ("events", state.events.to_string()),
                (
                    "last",
                    state.last.map_or_else(|| "—".into(), |v| v.to_string()),
                ),
                ("root_disabled", state.root_disabled.to_string()),
                (
                    "header_disabled",
                    WidgetryCollapsible::header_disabled(world, root)?.to_string(),
                ),
            ],
        );
        let rendered = binding.render(*world.resource::<Language>());
        if world
            .get::<Text>(status)
            .ok_or_else(|| demo_error("Collapsible 状态文字缺失"))?
            .0
            != rendered
        {
            world.entity_mut(status).insert((binding, Text(rendered)));
        }
    }
    Ok(())
}

fn demo_error(reason: &str) -> BevyError {
    error!(reason, "Collapsible 演示结构无效");
    BevyError::error(reason.to_owned())
}
