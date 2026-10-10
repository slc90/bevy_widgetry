use crate::pages;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
use bevy::ui::UiSystems;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};
pub(crate) struct GalleryPlugin;

#[derive(Component)]
struct GallerySidebar;

#[derive(Component)]
struct GalleryNavButton(GalleryPage);

#[derive(Component)]
struct GalleryPageContent(GalleryPage);

#[derive(Component)]
struct GalleryDemoContent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GalleryPage {
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

pub(crate) fn scene(
    list_sources: [Entity; 4],
    combo_sources: [Entity; 4],
    tree_sources: [Entity; 4],
    table_sources: pages::TableDemoSources,
    waveform_page: Box<dyn SceneList>,
) -> impl Scene {
    // 页面分别擦除 SceneList 类型，避免大型 BSN 组合在 Windows 主线程耗尽 stack。
    let button_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::button()});
    let check_box_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::check_box()});
    let combo_box_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::combo_box(combo_sources)});
    let scroll_area_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::scroll_area()});
    let list_view_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::list_view(list_sources)});
    let tree_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::tree(tree_sources)});
    let table_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::table(table_sources)});
    let text_field_page: Box<dyn SceneList> = Box::new(pages::text_field());
    let tooltip_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::tooltip()});
    let window_page: Box<dyn SceneList> = Box::new(bsn_list! {@pages::window()});
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
                Node { flex_grow: 1.0, min_width: px(0) }
                Children [
                    #ButtonPage @page(GalleryPage::Button, button_page)--
                    #CheckBoxPage @page(GalleryPage::CheckBox, check_box_page)--
                    #ComboBoxPage @page(GalleryPage::ComboBox, combo_box_page)--
                    #ScrollAreaPage @page(GalleryPage::ScrollArea, scroll_area_page)--
                    #ListViewPage @page(GalleryPage::ListView, list_view_page)--
                    #TreePage @page(GalleryPage::Tree, tree_page)--
                    #TablePage @page(GalleryPage::Table, table_page)--
                    #TextFieldPage @page(GalleryPage::TextField, text_field_page)--
                    #TooltipPage @page(GalleryPage::Tooltip, tooltip_page)--
                    #WindowPage @page(GalleryPage::Window, window_page)--
                    #WaveformPage @page(GalleryPage::Waveform, waveform_page)
                ]

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
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(16)),
            display: {if target == GalleryPage::Button { Display::Flex } else { Display::None }},
        }
        Children [{content}]
    }
}

fn on_nav_button_activated(
    event: On<Activate>,
    nav_buttons: Query<&GalleryNavButton>,
    mut pages: Query<(&GalleryPageContent, &mut Node)>,
) {
    let Ok(nav) = nav_buttons.get(event.entity) else {
        return;
    };

    info!(page = ?nav.0, "切换 Gallery 页面");
    for (page, mut node) in &mut pages {
        node.display = if page.0 == nav.0 {
            Display::Flex
        } else {
            Display::None
        };
    }
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
        app.add_systems(PostUpdate, constrain_demo_content.before(UiSystems::Layout));
        app.add_observer(refresh_sidebar_theme).add_plugins((
            pages::CheckBoxDemoPlugin,
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
