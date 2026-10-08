use crate::pages;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::prelude::*;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GalleryPage {
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
    let scroll_area_page: Box<dyn SceneList> = Box::new(bsn_list![pages::scroll_area()]);
    // 大页面使用 type-erased SceneList，避免 Gallery 初始 BSN 组合在 Windows 主线程耗尽 stack。
    let tree_page: Box<dyn SceneList> = Box::new(bsn_list![pages::tree(tree_sources)]);
    let table_page: Box<dyn SceneList> = Box::new(bsn_list![pages::table(table_sources)]);
    bsn! {
        #GalleryRoot
        TabGroup::default()
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
        }
        Children [
            (
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
                    (#ButtonNav navigation_button(GalleryPage::Button, "Button")),
                    (#CheckBoxNav navigation_button(GalleryPage::CheckBox, "CheckBox")),
                    (#ComboBoxNav navigation_button(GalleryPage::ComboBox, "ComboBox")),
                    (#ScrollAreaNav navigation_button(GalleryPage::ScrollArea, "ScrollArea")),
                    (#ListViewNav navigation_button(GalleryPage::ListView, "ListView")),
                    (#TreeNav navigation_button(GalleryPage::Tree, "Tree")),
                    (#TableNav navigation_button(GalleryPage::Table, "Table")),
                    (#TextFieldNav navigation_button(GalleryPage::TextField, "TextField")),
                    (#TooltipNav navigation_button(GalleryPage::Tooltip, "Tooltip")),
                    (#WindowNav navigation_button(GalleryPage::Window, "Window")),
                    (#WaveformNav navigation_button(GalleryPage::Waveform, "Waveform")),
                ]
            ),
            (
                #PageHost
                Node { flex_grow: 1.0, min_width: px(0) }
                Children [
                    (#ButtonPage page(GalleryPage::Button, bsn_list![pages::button()])),
                    (#CheckBoxPage page(GalleryPage::CheckBox, bsn_list![pages::check_box()])),
                    (#ComboBoxPage page(GalleryPage::ComboBox, bsn_list![pages::combo_box(combo_sources)])),
                    (#ScrollAreaPage page(GalleryPage::ScrollArea, scroll_area_page)),
                    (#ListViewPage page(GalleryPage::ListView, bsn_list![pages::list_view(list_sources)])),
                    (#TreePage page(GalleryPage::Tree, tree_page)),
                    (#TablePage page(GalleryPage::Table, table_page)),
                    (#TextFieldPage page(GalleryPage::TextField, pages::text_field())),
                    (#TooltipPage page(GalleryPage::Tooltip, bsn_list![pages::tooltip()])),
                    (#WindowPage page(GalleryPage::Window, bsn_list![pages::window()])),
                    (#WaveformPage page(GalleryPage::Waveform, waveform_page)),
                ]
            ),
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
        Children [Text(label)]
    }
}

fn page(target: GalleryPage, content: impl SceneList) -> impl Scene {
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

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_sidebar_theme).add_plugins((
            pages::ButtonDemoPlugin,
            pages::CheckBoxDemoPlugin,
            pages::ScrollAreaDemoPlugin,
            pages::ListViewDemoPlugin,
            pages::TreeDemoPlugin,
            pages::TableDemoPlugin,
            pages::ComboBoxDemoPlugin,
            pages::TooltipDemoPlugin,
            pages::WindowDemoPlugin,
            pages::WaveformDemoPlugin,
        ));
    }
}
