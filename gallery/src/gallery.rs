use crate::pages;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::StyledButton;

/// 将导航按钮绑定到目标页面，不额外保存当前页状态。
#[derive(Component)]
struct GalleryNavButton(GalleryPage);

/// 标记常驻页面；显隐以同一实体的 Node.display 为准。
#[derive(Component)]
struct GalleryPageContent(GalleryPage);

/// Gallery 当前提供的三个控件演示分类。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GalleryPage {
    Button,
    ComboBox,
    TextField,
}

/// 返回窗口内容区使用的 Gallery 场景。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        #GalleryRoot
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Row,
        }
        Children [
            (
                #Sidebar
                Node {
                    width: px(160),
                    flex_shrink: 0.0,
                    flex_direction: FlexDirection::Column,
                }
                Children [
                    (#ButtonNav navigation_button(GalleryPage::Button, "Button")),
                    (#ComboBoxNav navigation_button(GalleryPage::ComboBox, "ComboBox")),
                    (#TextFieldNav navigation_button(GalleryPage::TextField, "TextField")),
                ]
            ),
            (
                #PageHost
                Node { flex_grow: 1.0, min_width: px(0) }
                Children [
                    (#ButtonPage page(GalleryPage::Button, bsn_list![pages::button()])),
                    (#ComboBoxPage page(GalleryPage::ComboBox, bsn_list![pages::combo_box()])),
                    (#TextFieldPage page(GalleryPage::TextField, pages::text_field())),
                ]
            ),
        ]
    }
}

/// 用同一控件事件处理鼠标和键盘激活；标签只作为一次性的场景内容。
fn navigation_button(target: GalleryPage, label: &'static str) -> impl Scene {
    bsn! {
        template(|_| Ok(StyledButton))
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

/// 页面容器始终存在，初始只展开 Button 演示。
fn page(target: GalleryPage, content: impl SceneList) -> impl Scene {
    bsn! {
        template(move |_| Ok(GalleryPageContent(target)))
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            display: {if target == GalleryPage::Button { Display::Flex } else { Display::None }},
        }
        Children [{content}]
    }
}

/// 响应导航激活，只修改页面显隐以保留子控件状态。
fn on_nav_button_activated(
    event: On<Activate>,
    nav_buttons: Query<&GalleryNavButton>,
    mut pages: Query<(&GalleryPageContent, &mut Node)>,
) {
    let Ok(nav) = nav_buttons.get(event.entity) else {
        return;
    };

    for (page, mut node) in &mut pages {
        node.display = if page.0 == nav.0 {
            Display::Flex
        } else {
            Display::None
        };
    }
}
