use crate::pages;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::scene::spawn_scene;
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};
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
    CheckBox,
    ComboBox,
    ScrollArea,
    ListView,
    Tree,
    Table,
    TextField,
    Tooltip,
    Window,
    Waveform,
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #GalleryRoot
        TabGroup::default()
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
        }
        Children [

                #Sidebar
                template(|_| Ok(GallerySidebar))
                template(|context| Ok(BorderColor::all(context.resource::<WidgetryThemeMode>().colors().window.frame.normal.border)))
                Node {
                    width: px(176),
                    border: UiRect::right(px(1)),
                    padding: UiRect::all(px(16)),
                    row_gap: px(10),
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                }
                Children [
                    #ButtonNav @navigation_button(GalleryPage::Button, "Button")--
                    #CheckBoxNav @navigation_button(GalleryPage::CheckBox, "CheckBox")--
                    #ComboBoxNav @navigation_button(GalleryPage::ComboBox, "ComboBox")--
                    #ScrollAreaNav @navigation_button(GalleryPage::ScrollArea, "ScrollArea")--
                    #ListViewNav @navigation_button(GalleryPage::ListView, "ListView")--
                    #TreeNav @navigation_button(GalleryPage::Tree, "Tree")--
                    #TableNav @navigation_button(GalleryPage::Table, "Table")--
                    #TextFieldNav @navigation_button(GalleryPage::TextField, "TextField")--
                    #TooltipNav @navigation_button(GalleryPage::Tooltip, "Tooltip")--
                    #WindowNav @navigation_button(GalleryPage::Window, "Window")--
                    #WaveformNav @navigation_button(GalleryPage::Waveform, "Waveform")
                ]
            --

                #PageHost
                template(|_| Ok(GalleryPageHost))
                Node { flex_grow: 1.0, min_width: px(0) }

        ]
    }
}

fn navigation_button(target: GalleryPage, label: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(GalleryNavButton(target)))
        Node {
            width: percent(100),
            height: px(40),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        on(on_nav_button_activated)
        Children [Text(label) bevy_widgetry::text::WidgetryText]
    }
}

fn page(target: GalleryPage, content: impl SceneList) -> impl Scene {
    // 深层 SceneList 保持类型擦除，避免 Windows 主线程展开大型嵌套 BSN 时耗尽 stack。
    let content: Box<dyn SceneList> = if std::env::var_os("GALLERY_WAVEFORM_BENCH_OUTPUT").is_some()
        || std::env::var_os("GALLERY_FILE_DIALOG_BENCH_OUTPUT").is_some()
    {
        Box::new(content)
    } else {
        Box::new(bsn_list! {
            @crate::color_showcase::scene(target)--
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
            padding: UiRect::all(px(16)),
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
        error!(?target, "Initializing 不能挂载 Gallery 页面");
        return Err(BevyError::error("Initializing 不能挂载 Gallery 页面"));
    }
    let host = world
        .query_filtered::<Entity, With<GalleryPageHost>>()
        .single(world)
        .map_err(|error| {
            error!(%error, "Gallery 必须存在唯一 PageHost");
            BevyError::error(error)
        })?;
    if world
        .query_filtered::<Entity, With<GalleryPageContent>>()
        .iter(world)
        .next()
        .is_some()
    {
        error!(?target, "挂载 Gallery 页面前必须先销毁旧页面");
        return Err(BevyError::error("挂载 Gallery 页面前必须先销毁旧页面"));
    }
    spawn_scene(world, bsn! { ChildOf(host) @page(target, content) }).map_err(|error| {
        error!(?target, %error, "Gallery 页面构造失败");
        BevyError::error(error)
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
            error!(?target, ?roots, "退出 Gallery 页面时页面 root 不匹配");
            return Err(BevyError::error("退出 Gallery 页面时页面 root 不匹配"));
        }
    };
    world.despawn(root);
    Ok(())
}

fn page_content(world: &World, target: GalleryPage) -> Box<dyn SceneList> {
    match target {
        GalleryPage::Initializing => Box::new(bsn_list! {}),
        GalleryPage::Button => Box::new(bsn_list! {}),
        GalleryPage::CheckBox => Box::new(bsn_list! {}),
        GalleryPage::ComboBox => Box::new(bsn_list! {
            @pages::combo_box(world.resource::<pages::ComboBoxDemoSources>().0)
        }),
        GalleryPage::ScrollArea => Box::new(bsn_list! {}),
        GalleryPage::ListView => Box::new(bsn_list! {
            @pages::list_view(world.resource::<pages::ListViewDemoSources>().0)
        }),
        GalleryPage::Tree => Box::new(bsn_list! {
            @pages::tree(world.resource::<pages::TreeDemoSources>().0)
        }),
        GalleryPage::Table => Box::new(bsn_list! {
            @pages::table(world.resource::<pages::TableDemoSources>().clone())
        }),
        GalleryPage::TextField => Box::new(bsn_list! {}),
        GalleryPage::Tooltip => Box::new(bsn_list! {}),
        GalleryPage::Window => Box::new(bsn_list! {@pages::window()}),
        GalleryPage::Waveform => Box::new(bsn_list! {
            @pages::waveform(world.resource::<pages::WaveformDemoSources>())
        }),
    }
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
    if *current_page.get() == nav.0 {
        return;
    }

    info!(page = ?nav.0, "请求切换 Gallery 页面");
    next_page.set_if_different(nav.0);
}

fn refresh_sidebar_theme(
    event: On<WidgetryThemeChanged>,
    mut sidebars: Query<&mut BorderColor, With<GallerySidebar>>,
) {
    for mut border in &mut sidebars {
        *border = BorderColor::all(event.mode.colors().window.frame.normal.border);
    }
}

fn constrain_demo_content(mut contents: Query<&mut Node, Added<GalleryDemoContent>>) {
    // ScrollArea 的自然宽度会被 grid 内不换行的 controls 撑大。
    // 在首次 layout 前约束为 viewport 宽度，保留原有 grid 的可用列宽。
    for mut node in &mut contents {
        node.width = percent(100);
        node.max_width = percent(100);
    }
}

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GalleryPage>()
            .register_type::<State<GalleryPage>>()
            .add_systems(PostUpdate, constrain_demo_content.before(UiSystems::Layout));
        for target in [
            GalleryPage::ComboBox,
            GalleryPage::ListView,
            GalleryPage::Tree,
            GalleryPage::Table,
            GalleryPage::Window,
            GalleryPage::Waveform,
        ] {
            app.add_systems(OnEnter(target), move |world: &mut World| -> Result {
                let content = page_content(world, target);
                mount_page(world, target, content)?;
                Ok(())
            })
            .add_systems(OnExit(target), move |world: &mut World| {
                unmount_page(world, target)
            });
        }
        app.add_observer(refresh_sidebar_theme).add_plugins((
            pages::ButtonDemoPlugin,
            pages::CheckBoxDemoPlugin,
            pages::ScrollAreaDemoPlugin,
            pages::TextFieldDemoPlugin,
            pages::TooltipDemoPlugin,
            pages::ListViewDemoPlugin,
            pages::TreeDemoPlugin,
            pages::TableDemoPlugin,
            pages::ComboBoxDemoPlugin,
            pages::WindowDemoPlugin,
            pages::WaveformDemoPlugin,
            crate::color_showcase::ColorShowcasePlugin,
        ));
    }
}
