use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{
    LocalizedText, NavigationKey, TabsKey, TextKey, localized, localized_args, localized_binding,
};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::{
    button::WidgetryButton,
    scroll_area::WidgetryScrollArea,
    tabs::{
        TabsOrientation, WidgetryTab, WidgetryTabColorOverrides, WidgetryTabs,
        WidgetryTabsColorOverrides,
    },
    text::WidgetryText,
    text_field::WidgetryTextField,
    tooltip::{TooltipContentFactory, WidgetryTooltip},
};

const TAB_COUNT: usize = 10;

pub(crate) struct TabsDemoPlugin;

impl Plugin for TabsDemoPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TabsDemoState>()
            .add_systems(OnEnter(GalleryPage::Tabs), enter)
            .add_systems(OnExit(GalleryPage::Tabs), exit)
            .add_systems(Update, shortcuts.run_if(in_state(GalleryPage::Tabs)));
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Tabs, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Tabs)
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct TabsDemoState {
    notifications: u64,
    last_notification: Option<usize>,
}

#[derive(Component, FromTemplate)]
struct DemoParts {
    tabs: Entity,
    status: Entity,
}

#[derive(Component)]
struct Example(TabsOrientation);

#[derive(Component)]
struct DisabledRequest(bool);

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    First,
    Last,
    Reselect,
    RootDisabled,
    HeaderDisabled,
    Space,
}

fn scene() -> impl Scene {
    let demos: Vec<Box<dyn Scene>> = vec![
        Box::new(demo(TabsOrientation::Horizontal, "TabsHorizontal")),
        Box::new(demo(TabsOrientation::Vertical, "TabsVertical")),
    ];
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(18), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Navigation(NavigationKey::Tabs)) WidgetryText--
            @localized(TextKey::Tabs(TabsKey::Description)) WidgetryText Node { max_width: percent(100) }--
            {demos}
        ]
    }
}

fn demo(orientation: TabsOrientation, prefix: &'static str) -> impl Scene {
    let title = if orientation == TabsOrientation::Horizontal {
        TabsKey::Horizontal
    } else {
        TabsKey::Vertical
    };
    let controls: Vec<_> = [
        (TabsKey::First, Action::First),
        (TabsKey::Last, Action::Last),
        (TabsKey::Reselect, Action::Reselect),
        (TabsKey::ToggleRoot, Action::RootDisabled),
        (TabsKey::ToggleHeader, Action::HeaderDisabled),
        (TabsKey::ToggleSpace, Action::Space),
    ].into_iter().map(|(label, action)| bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Tabs(label)) WidgetryText }) as Box<dyn SceneList>)} }
        template(move |_| Ok(action)) Name({format!("{prefix}{action:?}")}) on(operate)
    }).collect();
    let tabs: Box<dyn Scene> = Box::new(bsn! {
        @WidgetryTabs { @orientation: orientation, @children: {Some(Box::new(items(orientation, prefix)) as Box<dyn SceneList>)} }
        template(move |_| Ok(Example(orientation)))
        template(|_| Ok(TabsDemoState::default()))
        template(|_| Ok(DisabledRequest(false)))
        Name({format!("{prefix}Root")})
        Node { width: px(580), max_width: percent(100), height: px(220), flex_shrink: 0.0 }
        on(selection_changed)
    });
    bsn! {
        DemoParts { tabs: #Tabs, status: #Status }
        Name({format!("{prefix}Demo")})
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), align_items: AlignItems::Start, flex_shrink: 0.0 }
        Children [
            @localized(TextKey::Tabs(title)) WidgetryText--
            Node { column_gap: px(8), row_gap: px(6), flex_wrap: FlexWrap::Wrap, max_width: percent(100) } Children [{controls}]--
            #Status Name({format!("{prefix}Status")})
            @localized_args(TextKey::Tabs(TabsKey::Status), status_args(0, 0, None, false, false))
            WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) } Node { max_width: percent(100) }--
            #Tabs @{tabs}
        ]
    }
}

fn items(orientation: TabsOrientation, prefix: &'static str) -> impl SceneList {
    let items: Vec<Box<dyn Scene>> = (0..TAB_COUNT).map(|index| {
        let binding = if orientation == TabsOrientation::Vertical && index == 2 {
            LocalizedText::new(TextKey::Tabs(TabsKey::LongTitle))
        } else {
            LocalizedText::with_args(TextKey::Tabs(TabsKey::Title), vec![("index", index.to_string())])
        };
        let tooltip = binding.clone();
        let disabled = (index == 1).then_some(bsn! { InteractionDisabled });
        let header: Box<dyn Scene> = Box::new(bsn! {
            Name({format!("{prefix}Header{index}")})
            template(move |_| Ok(DisabledRequest(index == 1)))
            @{disabled}
            @WidgetryTooltip { @content: {TooltipContentFactory::new(move || bsn_list! { @localized_binding(tooltip.clone()) WidgetryText })} }
            Children [@localized_binding(binding) WidgetryText]
        });
        let content: Box<dyn Scene> = Box::new(bsn! {
            @WidgetryScrollArea {
                @content: bsn! { Node { flex_direction: FlexDirection::Column, row_gap: px(12), padding: UiRect::all(px(16)) } },
                @children: bsn_list! {
                    @localized_args(TextKey::Tabs(TabsKey::Page), vec![("index", index.to_string())]) WidgetryText--
                    Name({format!("{prefix}Field{index}")}) @WidgetryTextField
                    bevy::input_focus::tab_navigation::TabIndex(0) Node { width: px(250), max_width: percent(100) }--
                    Node { height: px(420), width: percent(100), flex_shrink: 0.0 }
                    Children [@localized(TextKey::Tabs(TabsKey::Retained)) WidgetryText]
                },
            }
            Name({format!("{prefix}Content{index}")})
            Node { width: percent(100), height: percent(100), min_width: px(0), min_height: px(0) }
        });
        Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } }) as Box<dyn Scene>
    }).collect();
    bsn_list! { {items} }
}

fn status_args(
    index: usize,
    notifications: u64,
    last: Option<usize>,
    root: bool,
    header: bool,
) -> Vec<(&'static str, String)> {
    vec![
        ("index", index.to_string()),
        ("events", notifications.to_string()),
        (
            "last",
            last.map_or_else(|| "—".into(), |last| last.to_string()),
        ),
        ("root_disabled", root.to_string()),
        ("header_disabled", header.to_string()),
    ]
}

fn demo_error(reason: &str) -> BevyError {
    error!(reason, "Tabs 演示结构无效");
    BevyError::error(reason.to_owned())
}

fn report(world: &mut World, root: Entity) -> Result {
    let parent = world
        .get::<ChildOf>(root)
        .map(ChildOf::parent)
        .ok_or_else(|| demo_error("Tabs 演示父容器缺失"))?;
    let status = world
        .get::<DemoParts>(parent)
        .map(|parts| parts.status)
        .ok_or_else(|| demo_error("Tabs 演示状态标签缺失"))?;
    let index = WidgetryTabs::selected_index(world, root)?;
    let header = WidgetryTabs::header(world, root, index)?;
    let state = world
        .get::<TabsDemoState>(root)
        .ok_or_else(|| demo_error("Tabs 演示计数缺失"))?;
    let args = status_args(
        index,
        state.notifications,
        state.last_notification,
        world.get::<InteractionDisabled>(root).is_some(),
        world.get::<InteractionDisabled>(header).is_some(),
    );
    *world
        .get_mut::<LocalizedText>(status)
        .ok_or_else(|| demo_error("Tabs 本地化状态缺失"))? =
        LocalizedText::with_args(TextKey::Tabs(TabsKey::Status), args);
    Ok(())
}

fn selection_changed(
    event: On<ValueChange<usize>>,
    mut states: Query<&mut TabsDemoState>,
    mut commands: Commands,
) -> Result {
    let root = event.source;
    let mut state = states
        .get_mut(root)
        .map_err(|_| demo_error("Tabs 选择通知目标无效"))?;
    state.notifications += 1;
    state.last_notification = Some(event.value);
    info!(
        ?root,
        index = event.value,
        notifications = state.notifications,
        "Tabs 选择已改变"
    );
    commands.queue(move |world: &mut World| report(world, root));
    Ok(())
}

fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    parents: Query<&ChildOf>,
    demos: Query<&DemoParts>,
    mut commands: Commands,
) -> Result {
    let action = *actions
        .get(event.entity)
        .map_err(|_| demo_error("Tabs 操作类型缺失"))?;
    let root = parents
        .iter_ancestors(event.entity)
        .find_map(|parent| demos.get(parent).ok().map(|parts| parts.tabs))
        .ok_or_else(|| demo_error("Tabs 操作实例缺失"))?;
    commands.queue(move |world: &mut World| perform(world, root, action));
    Ok(())
}

fn perform(world: &mut World, root: Entity, action: Action) -> Result {
    let index = WidgetryTabs::selected_index(world, root)?;
    match action {
        Action::First | Action::Last | Action::Reselect => {
            let target = match action {
                Action::First => 0,
                Action::Last => TAB_COUNT - 1,
                _ => index,
            };
            let changed = WidgetryTabs::set_selected_in_world(world, root, target)?;
            info!(?root, target, changed, "程序选择 Tabs 标签");
        }
        Action::RootDisabled | Action::HeaderDisabled => {
            let target = if matches!(action, Action::RootDisabled) {
                root
            } else {
                WidgetryTabs::header(world, root, index)?
            };
            let disabled = !world
                .get::<DisabledRequest>(target)
                .ok_or_else(|| demo_error("Tabs 禁用请求缺失"))?
                .0;
            if !disabled {
                world.entity_mut(target).remove::<InteractionDisabled>();
            } else {
                world.entity_mut(target).insert(InteractionDisabled);
            }
            world.entity_mut(target).insert(DisabledRequest(disabled));
            info!(?root, ?target, ?action, disabled, "切换 Tabs 禁用请求");
        }
        Action::Space => {
            let vertical = world
                .get::<Example>(root)
                .is_some_and(|example| example.0 == TabsOrientation::Vertical);
            let mut node = world
                .get_mut::<Node>(root)
                .ok_or_else(|| demo_error("Tabs 布局缺失"))?;
            let roomy = node.width == px(580);
            node.width = if roomy { percent(100) } else { px(580) };
            node.height = if roomy && vertical { px(430) } else { px(220) };
            info!(?root, roomy, "切换 Tabs 演示可用空间");
        }
    }
    report(world, root)
}

fn shortcuts(
    input: Res<ButtonInput<KeyCode>>,
    roots: Query<(Entity, &Example)>,
    mut commands: Commands,
) {
    for (root, example) in &roots {
        let key = if example.0 == TabsOrientation::Horizontal {
            KeyCode::F8
        } else {
            KeyCode::F9
        };
        if input.just_pressed(key) {
            commands.queue(move |world: &mut World| perform(world, root, Action::Last));
        }
    }
}

pub(crate) fn color_examples() -> impl Scene {
    crate::color_showcase::pair(
        TextKey::Navigation(NavigationKey::Tabs),
        TextKey::Tabs(TabsKey::Overrides),
        color_sample(),
        color_sample(),
        apply_colors,
    )
}

fn color_sample() -> impl Scene {
    let items: Vec<Box<dyn Scene>> = (0..2).map(|index| {
        Box::new(bsn! { @WidgetryTab {
            @header: {Some(Box::new(bsn! { Children [Text({index.to_string()}) WidgetryText] }) as Box<dyn Scene>)},
            @content: {Some(Box::new(bsn! { @localized_args(TextKey::Tabs(TabsKey::Page), vec![("index", index.to_string())]) WidgetryText }) as Box<dyn Scene>)},
        } }) as Box<dyn Scene>
    }).collect();
    bsn! {
        @WidgetryTabs { @children: {Some(Box::new(items) as Box<dyn SceneList>)} }
        Node { width: percent(100), height: px(100), min_width: px(0) }
    }
}

fn apply_colors(world: &mut World, root: Entity, apply: bool) -> Result<bool> {
    let local = WidgetryTabs::header(world, root, 1)?;
    let mut group = WidgetryTabsColorOverrides::default();
    let mut header = WidgetryTabColorOverrides::default();
    if apply {
        group.header.selected.normal.background = Some(crate::color_showcase::TEAL);
        group.header.selected.hovered.background = Some(crate::color_showcase::TEAL);
        header.selected.normal.background = Some(crate::color_showcase::PURPLE);
        header.selected.hovered.background = Some(crate::color_showcase::PURPLE);
    }
    let changed = WidgetryTabsColorOverrides::set_in_world(world, root, group)?;
    let local_changed = WidgetryTabColorOverrides::set_in_world(world, local, header)?;
    Ok(changed || local_changed)
}
