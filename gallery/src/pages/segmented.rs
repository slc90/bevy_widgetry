const CYCLE_OPTION_COUNT: usize = 3;
const EQUAL_OPTIONS_WIDTH: f32 = 360.0;
const PANEL_WIDTH: f32 = 440.0;

const DETAIL_GAP: f32 = 6.0;
const ITEM_GAP: f32 = 8.0;
const GROUP_GAP: f32 = 12.0;
const SECTION_GAP: f32 = 16.0;
const PAGE_GAP: f32 = 20.0;
const CONTROL_GAP: f32 = 16.0;
const CONTENT_GAP: f32 = 24.0;
const LABEL_FONT_SIZE: f32 = 14.0;

use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{LocalizedText, NavigationKey, SegmentedKey, TextKey, localized, localized_args};
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::{
    button::WidgetryButton,
    segmented::{
        SegmentedOrientation, WidgetrySegmented, WidgetrySegmentedColorOverrides,
        WidgetrySegmentedHeadless, WidgetrySegmentedOption, WidgetrySegmentedOptionColorOverrides,
    },
    text::WidgetryText,
};

mod custom;

pub(crate) struct SegmentedDemoPlugin;

impl Plugin for SegmentedDemoPlugin {
    fn build(&self, app: &mut App) {
        custom::install(app);
        app.register_type::<SegmentedDemoState>()
            .add_systems(OnEnter(GalleryPage::Segmented), enter)
            .add_systems(OnExit(GalleryPage::Segmented), exit);
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::Segmented, bsn_list! { @scene() })?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Segmented)
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct SegmentedDemoState {
    user_events: u64,
    programmatic: u64,
}

#[derive(Component)]
struct Status;

#[derive(Component)]
struct ProgrammaticTarget;

fn scene() -> impl Scene {
    let examples: Vec<Box<dyn Scene>> = vec![
        Box::new(demo(
            SegmentedKey::Natural,
            "SegmentedNatural",
            bsn! {
                @WidgetrySegmented Children [{date_options("SegmentedNatural")}]
            },
            0,
        )),
        Box::new(demo(
            SegmentedKey::Equal,
            "SegmentedEqual",
            bsn! {
                @WidgetrySegmented { @equal_width: true } Node { width: px(EQUAL_OPTIONS_WIDTH) }
                Children [{date_options("SegmentedEqual")}]
            },
            0,
        )),
        Box::new(demo(
            SegmentedKey::OptionDisabled,
            "SegmentedOptionDisabled",
            bsn! {
                @WidgetrySegmented Children [{disabled_options("SegmentedOptionDisabled", false)}]
            },
            0,
        )),
        Box::new(demo(
            SegmentedKey::SelectedDisabled,
            "SegmentedSelectedDisabled",
            bsn! {
                @WidgetrySegmented Children [{disabled_options("SegmentedSelectedDisabled", true)}]
            },
            1,
        )),
        Box::new(demo(
            SegmentedKey::GroupDisabled,
            "SegmentedGroupDisabled",
            bsn! {
                @WidgetrySegmented InteractionDisabled template(|_| Ok(ProgrammaticTarget))
                Children [{date_options("SegmentedGroupDisabled")}]
            },
            0,
        )),
        Box::new(demo(
            SegmentedKey::Vertical,
            "SegmentedVertical",
            bsn! {
                @WidgetrySegmented { @orientation: SegmentedOrientation::Vertical }
                Children [{disabled_options("SegmentedVertical", false)}]
            },
            0,
        )),
    ];
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(SECTION_GAP), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Navigation(NavigationKey::Segmented)) WidgetryText--
            @localized(TextKey::Segmented(SegmentedKey::Description)) WidgetryText Node { max_width: percent(100) }--
            Node { column_gap: px(CONTROL_GAP), row_gap: px(GROUP_GAP), flex_wrap: FlexWrap::Wrap, align_items: AlignItems::Center }
            Children [
                @action(SegmentedKey::FocusBefore, "SegmentedFocusBefore") bevy::input_focus::tab_navigation::TabIndex(0)--
                @demo(SegmentedKey::Basic, "SegmentedBasic", bsn! {
                    @WidgetrySegmented Children [
                        @option("SegmentedBasic0", SegmentedKey::Day)--
                        @option("SegmentedBasic1", SegmentedKey::Week)--
                        @option("SegmentedBasic2", SegmentedKey::Month)--
                        @option("SegmentedBasic3", SegmentedKey::Year)
                    ]
                }, 0)--
                @action(SegmentedKey::FocusAfter, "SegmentedFocusAfter") bevy::input_focus::tab_navigation::TabIndex(0)
            ]--
            Node { column_gap: px(CONTROL_GAP), row_gap: px(ITEM_GAP), flex_wrap: FlexWrap::Wrap }
            Children [
                @action(SegmentedKey::Programmatic, "SegmentedSelectNext") on(select_next)--
                @action(SegmentedKey::ToggleDisabled, "SegmentedToggleDisabled") on(toggle_disabled)
            ]--
            Node { column_gap: px(CONTENT_GAP), row_gap: px(PAGE_GAP), flex_wrap: FlexWrap::Wrap, align_items: AlignItems::Start }
            Children [{examples}]--
            @custom::scene()
        ]
    }
}

fn demo(
    title: SegmentedKey,
    name: &'static str,
    control: impl Scene,
    initial: usize,
) -> impl Scene {
    let control: Box<dyn Scene> = Box::new(control);
    bsn! {
        template(|_| Ok(SegmentedDemoState::default())) Name({format!("{name}Demo")})
        Node { width: px(PANEL_WIDTH), max_width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(DETAIL_GAP), align_items: AlignItems::Start }
        Children [
            @localized(TextKey::Segmented(title)) WidgetryText--
            @{control} Name(name) on(selection_changed)--
            template(|_| Ok(Status)) Name({format!("{name}Status")})
            @localized_args(TextKey::Segmented(SegmentedKey::Status), vec![("index", initial.to_string()), ("events", "0".into()), ("programmatic", "0".into())])
            WidgetryText TextFont { font_size: bevy::text::FontSize::Px(LABEL_FONT_SIZE) }
        ]
    }
}

fn option(name: impl Into<String>, label: SegmentedKey) -> impl Scene {
    let name = name.into();
    bsn! { @WidgetrySegmentedOption Name(name) Children [@localized(TextKey::Segmented(label)) WidgetryText] }
}

fn date_options(prefix: &'static str) -> impl SceneList {
    bsn_list! {
        @option(format!("{prefix}0"), SegmentedKey::Today)--
        @option(format!("{prefix}1"), SegmentedKey::Last7)--
        @option(format!("{prefix}2"), SegmentedKey::Last30)
    }
}

fn disabled_options(prefix: &'static str, selected_disabled: bool) -> impl SceneList {
    bsn_list! {
        @option(format!("{prefix}0"), SegmentedKey::Day)--
        @option(format!("{prefix}1"), SegmentedKey::Week) InteractionDisabled
        @{selected_disabled.then_some(bsn! { Checked })}--
        @option(format!("{prefix}2"), SegmentedKey::Month)
    }
}

fn action(label: SegmentedKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! { @localized(TextKey::Segmented(label)) WidgetryText }) as Box<dyn SceneList>)} }
        Name(name)
    }
}

fn selection_changed(event: On<ValueChange<usize>>, mut commands: Commands) {
    let root = event.source;
    commands.queue(move |world: &mut World| report(world, root, true));
}

fn report(world: &mut World, root: Entity, user: bool) -> Result {
    let result = (|| -> Result<usize> {
        let parent = world
            .get::<ChildOf>(root)
            .map(ChildOf::parent)
            .ok_or_else(|| crate::diagnostics::error("Segmented 演示缺失父容器"))?;
        let index = selected_index(world, root)?;
        let (events, programmatic) = {
            let mut state = world
                .get_mut::<SegmentedDemoState>(parent)
                .ok_or_else(|| crate::diagnostics::error("Segmented 演示状态缺失"))?;
            if user {
                state.user_events += 1;
            } else {
                state.programmatic += 1;
            }
            (state.user_events, state.programmatic)
        };
        let label = world
            .get::<Children>(parent)
            .and_then(|children| {
                children
                    .iter()
                    .find(|child| world.get::<Status>(*child).is_some())
            })
            .ok_or_else(|| crate::diagnostics::error("Segmented 演示状态文字缺失"))?;
        let mut text = world
            .get_mut::<LocalizedText>(label)
            .ok_or_else(|| crate::diagnostics::error("Segmented 演示本地化文字缺失"))?;
        *text = LocalizedText::with_args(
            TextKey::Segmented(SegmentedKey::Status),
            vec![
                ("index", index.to_string()),
                ("events", events.to_string()),
                ("programmatic", programmatic.to_string()),
            ],
        );
        Ok(index)
    })();
    match result {
        Ok(index) => {
            info!(?root, index, user, "更新 Segmented 演示选择");
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn selected_index(world: &World, root: Entity) -> Result<usize> {
    world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .position(|child| world.get::<Checked>(child).is_some())
        })
        .ok_or_else(|| crate::diagnostics::error("Segmented 演示缺失选中项"))
}

fn select_next(
    _: On<Activate>,
    targets: Query<Entity, With<ProgrammaticTarget>>,
    mut commands: Commands,
) {
    for root in &targets {
        commands.queue(move |world: &mut World| -> Result {
            let index = selected_index(world, root)?;
            WidgetrySegmentedHeadless::set_selected(
                &mut world.commands(),
                root,
                (index + 1) % CYCLE_OPTION_COUNT,
            );
            world
                .commands()
                .queue(move |world: &mut World| report(world, root, false));
            Ok(())
        });
    }
}

fn toggle_disabled(
    _: On<Activate>,
    targets: Query<Entity, With<ProgrammaticTarget>>,
    mut commands: Commands,
) {
    for root in &targets {
        commands.queue(move |world: &mut World| {
            let disabled = world.get::<InteractionDisabled>(root).is_some();
            if disabled {
                world.entity_mut(root).remove::<InteractionDisabled>();
            } else {
                world.entity_mut(root).insert(InteractionDisabled);
            }
            info!(?root, disabled = !disabled, "切换 Segmented 整组禁用");
        });
    }
}

pub(crate) fn color_examples() -> impl Scene {
    crate::color_showcase::pair(
        TextKey::Navigation(NavigationKey::Segmented),
        TextKey::Segmented(SegmentedKey::Overrides),
        color_sample(),
        color_sample(),
        apply_colors,
    )
}

fn color_sample() -> impl Scene {
    bsn! { @WidgetrySegmented Children [{date_options("SegmentedColor")}] }
}

fn apply_colors(world: &mut World, root: Entity, apply: bool) -> Result<bool> {
    let local = world
        .get::<Children>(root)
        .and_then(|children| children.get(1))
        .copied()
        .ok_or_else(|| {
            crate::diagnostics::at(
                "Segmented 颜色示例缺失中间项",
                bevy_widgetry::diagnostics::ErrorFrame::new("Segmented 颜色示例缺失中间项")
                    .field("root", format_args!("{:?}", root)),
            )
        })?;
    let mut group = WidgetrySegmentedColorOverrides::default();
    let mut option = WidgetrySegmentedOptionColorOverrides::default();
    if apply {
        group.option.checked.normal.background = Some(crate::color_showcase::TEAL);
        group.option.checked.hovered.background = Some(crate::color_showcase::TEAL);
        option.checked.normal.background = Some(crate::color_showcase::PURPLE);
        option.checked.hovered.background = Some(crate::color_showcase::PURPLE);
    }
    let group_changed = WidgetrySegmentedColorOverrides::set_in_world(world, root, group)
        .map_err(|error| error.into_bevy_error())?;
    let local_changed = WidgetrySegmentedOptionColorOverrides::set_in_world(world, local, option)
        .map_err(|error| error.into_bevy_error())?;
    Ok(group_changed || local_changed)
}
