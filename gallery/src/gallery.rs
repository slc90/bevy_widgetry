const SIDEBAR_WIDTH: f32 = 176.0;
const SIDEBAR_BORDER_WIDTH: f32 = 1.0;
const PAGE_PADDING: f32 = 16.0;
const NAV_BUTTON_HEIGHT: f32 = 40.0;

const ITEM_GAP: f32 = 10.0;

use crate::i18n::NavigationKey;
use crate::i18n::{TextKey, localized};
use crate::pages;
use bevy::prelude::*;
use bevy::state::state::{StateTransitionEvent, StateTransitionSystems};
use bevy::ui::UiSystems;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::scene::spawn_scene;
use bevy_widgetry::scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
};
use bevy_widgetry::theme::{WidgetryTheme, WidgetryThemeChanged};
pub(crate) struct GalleryPlugin;

#[derive(Component)]
struct GallerySidebar;

#[derive(Component)]
struct GalleryNavButton(GalleryPage);

#[derive(Component)]
pub(crate) struct GalleryPageHost;

#[derive(Component)]
pub(crate) struct GalleryPageContent(pub(crate) GalleryPage);

#[derive(Component)]
struct GalleryDemoContent;

#[derive(States, Default, Clone, Copy, Debug, PartialEq, Eq, Hash, Reflect)]
pub(crate) enum GalleryPage {
    #[default]
    Initializing,
    Button,
    Segmented,
    Tabs,
    Collapsible,
    CheckBox,
    ComboBox,
    DatePicker,
    ScrollArea,
    Slider,
    NumberInput,
    ProgressBar,
    Splitter,
    ListView,
    Tree,
    Table,
    TextField,
    Tooltip,
    Window,
    Waveform,
    Canvas,
    Job,
    PowerInhibit,
    Cursor,
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #GalleryRoot
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
        }
        Children [

                #Sidebar
                @WidgetryScrollArea {
                    @axis: ScrollAxis::Vertical,
                    @scrollbar_visibility: {ScrollbarVisibility { horizontal: ScrollbarPolicy::Auto, vertical: ScrollbarPolicy::Auto }},
                    @content: bsn! { Node { row_gap: px(ITEM_GAP), align_items: AlignItems::Center } },
                    @children: bsn_list! {
                        Name("ButtonNav") @navigation_button(GalleryPage::Button,TextKey::Navigation(NavigationKey::Button))--
                        Name("SegmentedNav") @navigation_button(GalleryPage::Segmented,TextKey::Navigation(NavigationKey::Segmented))--
                        Name("TabsNav") @navigation_button(GalleryPage::Tabs,TextKey::Navigation(NavigationKey::Tabs))--
                        Name("CollapsibleNav") @navigation_button(GalleryPage::Collapsible,TextKey::Navigation(NavigationKey::Collapsible))--
                        Name("CheckBoxNav") @navigation_button(GalleryPage::CheckBox,TextKey::Navigation(NavigationKey::CheckBox))--
                        Name("ComboBoxNav") @navigation_button(GalleryPage::ComboBox,TextKey::Navigation(NavigationKey::ComboBox))--
                        Name("DatePickerNav") @navigation_button(GalleryPage::DatePicker,TextKey::Navigation(NavigationKey::DatePicker))--
                        Name("ScrollAreaNav") @navigation_button(GalleryPage::ScrollArea,TextKey::Navigation(NavigationKey::ScrollArea))--
                        Name("ProgressBarNav") @navigation_button(GalleryPage::ProgressBar,TextKey::Navigation(NavigationKey::ProgressBar))--
                        Name("SliderNav") @navigation_button(GalleryPage::Slider,TextKey::Navigation(NavigationKey::Slider))--
                        Name("NumberInputNav") @navigation_button(GalleryPage::NumberInput,TextKey::Navigation(NavigationKey::NumberInput))--
                        Name("SplitterNav") @navigation_button(GalleryPage::Splitter,TextKey::Navigation(NavigationKey::Splitter))--
                        Name("ListViewNav") @navigation_button(GalleryPage::ListView,TextKey::Navigation(NavigationKey::ListView))--
                        Name("TreeNav") @navigation_button(GalleryPage::Tree,TextKey::Navigation(NavigationKey::Tree))--
                        Name("TableNav") @navigation_button(GalleryPage::Table,TextKey::Navigation(NavigationKey::Table))--
                        Name("TextFieldNav") @navigation_button(GalleryPage::TextField,TextKey::Navigation(NavigationKey::TextField))--
                        Name("TooltipNav") @navigation_button(GalleryPage::Tooltip,TextKey::Navigation(NavigationKey::Tooltip))--
                        Name("WindowNav") @navigation_button(GalleryPage::Window,TextKey::Navigation(NavigationKey::Window))--
                        Name("CanvasNav") @navigation_button(GalleryPage::Canvas,TextKey::Navigation(NavigationKey::Canvas))--
                        Name("WaveformNav") @navigation_button(GalleryPage::Waveform,TextKey::Navigation(NavigationKey::Waveform))--
                        Name("JobNav") @navigation_button(GalleryPage::Job,TextKey::Navigation(NavigationKey::Job))--
                        Name("PowerInhibitNav") @navigation_button(GalleryPage::PowerInhibit,TextKey::Navigation(NavigationKey::PowerInhibit))--
                        Name("CursorNav") @navigation_button(GalleryPage::Cursor,TextKey::Navigation(NavigationKey::Cursor))
                    },
                }
                template(|_| Ok(GallerySidebar))
                template(|context| Ok(BorderColor::all(context.resource::<WidgetryTheme>().window.frame.normal.border)))
                Node {
                    width: px(SIDEBAR_WIDTH),
                    height: percent(100),
                    min_height: px(0),
                    border: UiRect::right(px(SIDEBAR_BORDER_WIDTH)),
                    padding: UiRect::all(px(PAGE_PADDING)),
                    flex_shrink: 0.0,
                }
            --

                #PageHost
                template(|_| Ok(GalleryPageHost))
                Node { flex_grow: 1.0, min_width: px(0) }

        ]
    }
}

fn navigation_button(target: GalleryPage, label: TextKey) -> impl Scene {
    bsn! {
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(label) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        template(move |_| Ok(GalleryNavButton(target)))
        Node {
            width: percent(100),
            height: px(NAV_BUTTON_HEIGHT),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        on(on_nav_button_activated)

    }
}

fn page(target: GalleryPage, content: impl SceneList) -> impl Scene {
    let colors = (!matches!(
        target,
        GalleryPage::Canvas | GalleryPage::Job | GalleryPage::PowerInhibit
    ))
    .then(|| crate::color_showcase::scene(target));
    // Keep deep SceneLists type-erased so expanding large nested BSN cannot exhaust the Windows main thread stack.
    let content: Box<dyn SceneList> = if std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT").is_some()
        || std::env::var_os("GALLERY_FILE_DIALOG_BENCH_OUTPUT").is_some()
    {
        Box::new(content)
    } else {
        Box::new(bsn_list! {
            @{colors}--
            @bevy_widgetry::scroll_area::WidgetryScrollArea {
                @content: bsn! { template(|_| Ok(GalleryDemoContent)) },
                @children: bsn_list!{Node { width: percent(100), height: vh(100), flex_shrink: 0.0, flex_direction: FlexDirection::Column } Children [{content}]},
            }
                Name({format!("Gallery{target:?}Examples")})
                Node { width: percent(100), flex_grow: 1.0, min_height: px(0), min_width: px(0) }
        })
    };
    bsn! {
        template(move |_| Ok(GalleryPageContent(target)))
        Name({format!("{target:?}Page")})
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(PAGE_PADDING)),
        }
        Children [{content}]
    }
}

pub(crate) fn mount_page(
    world: &mut World,
    target: GalleryPage,
    content: impl SceneList,
) -> Result<Entity, BevyError> {
    if target == GalleryPage::Initializing {
        return Err(crate::diagnostics::at(
            "Initializing 不能挂载 Gallery 页面",
            bevy_widgetry::diagnostics::ErrorFrame::new("Initializing 不能挂载 Gallery 页面")
                .field("target", format_args!("{:?}", target)),
        ));
    }
    let host = world
        .query_filtered::<Entity, With<GalleryPageHost>>()
        .single(world)
        .map_err(|error| {
            crate::diagnostics::at(
                error,
                bevy_widgetry::diagnostics::ErrorFrame::new("Gallery 必须存在唯一 PageHost"),
            )
        })?;
    if world
        .query_filtered::<Entity, With<GalleryPageContent>>()
        .iter(world)
        .next()
        .is_some()
    {
        return Err(crate::diagnostics::at(
            "挂载 Gallery 页面前必须先销毁旧页面",
            bevy_widgetry::diagnostics::ErrorFrame::new("挂载 Gallery 页面前必须先销毁旧页面")
                .field("target", format_args!("{:?}", target)),
        ));
    }
    spawn_scene(world, bsn! { ChildOf(host) @page(target, content) }).map_err(|error| {
        error
            .with_context(
                bevy_widgetry::diagnostics::ErrorFrame::new("Gallery 页面构造")
                    .field("target", format!("{target:?}")),
            )
            .into_bevy_error()
    })
}

pub(crate) fn unmount_page(world: &mut World, target: GalleryPage) -> Result {
    let roots: Vec<_> = world
        .query::<(Entity, &GalleryPageContent)>()
        .iter(world)
        .map(|(entity, page)| (entity, page.0))
        .collect();
    let root = match roots.as_slice() {
        [] => return Ok(()),
        [(root, page)] if *page == target => *root,
        _ => {
            return Err(crate::diagnostics::at(
                "退出 Gallery 页面时页面 root 不匹配",
                bevy_widgetry::diagnostics::ErrorFrame::new("退出 Gallery 页面时页面 root 不匹配")
                    .field("target", format_args!("{:?}", target))
                    .field("roots", format_args!("{:?}", roots)),
            ));
        }
    };
    world.despawn(root);
    Ok(())
}

fn on_nav_button_activated(
    event: On<Activate>,
    nav_buttons: Query<&GalleryNavButton>,
    current_page: Res<State<GalleryPage>>,
    mut next_page: ResMut<NextState<GalleryPage>>,
) {
    let Ok(nav) = nav_buttons.get(event.entity) else {
        return;
    };
    let unchanged = *current_page.get() == nav.0;
    info!(from = ?current_page.get(), to = ?nav.0, unchanged, "用户选择 Gallery 页面");
    if unchanged {
        return;
    }

    next_page.set_if_different(nav.0);
}

fn log_page_transition(mut transitions: MessageReader<StateTransitionEvent<GalleryPage>>) {
    for transition in transitions.read() {
        if transition.exited != transition.entered {
            debug!(from = ?transition.exited, to = ?transition.entered, "Gallery 页面状态已切换");
        }
    }
}

fn refresh_sidebar_theme(
    _: On<WidgetryThemeChanged>,
    mode: Res<WidgetryTheme>,
    mut sidebars: Query<&mut BorderColor, With<GallerySidebar>>,
) {
    for mut border in &mut sidebars {
        *border = BorderColor::all(mode.window.frame.normal.border);
    }
}

fn constrain_demo_content(mut contents: Query<&mut Node, Added<GalleryDemoContent>>) {
    // Non-wrapping controls inside the grid can inflate ScrollArea's natural width.
    // Constrain it to the viewport width before the first layout to preserve the grid's available column width.
    for mut node in &mut contents {
        node.width = percent(100);
        node.max_width = percent(100);
    }
}

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GalleryPage>()
            .register_type::<State<GalleryPage>>()
            .add_systems(
                StateTransition,
                log_page_transition
                    .after(StateTransitionSystems::DependentTransitions)
                    .before(StateTransitionSystems::ExitSchedules),
            )
            .add_systems(PostUpdate, constrain_demo_content.before(UiSystems::Layout));
        app.add_observer(refresh_sidebar_theme).add_plugins((
            pages::ButtonDemoPlugin,
            (
                pages::SegmentedDemoPlugin,
                pages::TabsDemoPlugin,
                pages::CollapsibleDemoPlugin,
                pages::DatePickerDemoPlugin,
            ),
            pages::CheckBoxDemoPlugin,
            pages::ScrollAreaDemoPlugin,
            (
                pages::SliderDemoPlugin,
                pages::NumberInputDemoPlugin,
                pages::ProgressBarDemoPlugin,
                pages::SplitterDemoPlugin,
            ),
            pages::TextFieldDemoPlugin,
            pages::TooltipDemoPlugin,
            pages::ListViewDemoPlugin,
            pages::TreeDemoPlugin,
            pages::TableDemoPlugin,
            pages::ComboBoxDemoPlugin,
            pages::WindowDemoPlugin,
            pages::WaveformDemoPlugin,
            (
                pages::CursorDemoPlugin,
                pages::CanvasDemoPlugin,
                pages::JobDemoPlugin,
                pages::PowerInhibitDemoPlugin,
            ),
            crate::color_showcase::ColorShowcasePlugin,
        ));
    }
}
