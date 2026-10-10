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
pub(crate) struct GalleryPageHost;

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

fn on_nav_button_activated(
    event: On<Activate>,
    nav_buttons: Query<&GalleryNavButton>,
    mut next_page: ResMut<NextState<GalleryPage>>,
) {
    let Ok(nav) = nav_buttons.get(event.entity) else {
        return;
    };

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

fn verify_page_host(hosts: Query<Entity, With<GalleryPageHost>>) -> Result {
    hosts.single().map_err(|error| {
        error!(error = %error, "Gallery 必须存在唯一 PageHost");
        BevyError::error(error)
    })?;
    Ok(())
}

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GalleryPage>()
            .register_type::<State<GalleryPage>>()
            .add_systems(OnEnter(GalleryPage::Button), verify_page_host);
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
