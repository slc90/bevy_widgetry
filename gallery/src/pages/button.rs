use crate::assets::GalleryIcon;
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::{
    button::WidgetryButton,
    icon::WidgetryIcon,
    style::{ForegroundColor, ThemeChanged, ThemeMode},
};

/// 为按钮页的分组标题提供主题响应。
pub(crate) struct ButtonDemoPlugin;

/// 标记页面的主题前景色root，按钮仍使用自身状态配色。
#[derive(Component)]
struct ButtonDemo;

/// 对比普通与禁用状态的四种内容组合，悬停和按下由真实指针交互呈现。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        #ButtonDemo
        template(|_| Ok(ButtonDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
        Children [
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Normal"), button_row(false)]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Disabled"), button_row(true)]),
        ]
    }
}

/// 两组使用相同内容与布局，仅禁用组为每个按钮附加官方禁用状态。
fn button_row(disabled: bool) -> impl Scene {
    let buttons = bsn_list![
        (@WidgetryButton {}
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            Children [Text("Text")]),
        (@WidgetryButton {}
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) }
            Children [star(), Text("Icon + Text")]),
        (@WidgetryButton {}
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) }
            Children [Text("Text + Icon"), star()]),
        (@WidgetryButton {}
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { width: px(32), height: px(32), padding: UiRect::all(px(6)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            Children [star()]),
    ];
    bsn! {
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(12) }
        Children [{buttons}]
    }
}

/// 按钮内容图标继承按钮前景色。
/// SVG 的 currentColor 先生成白色遮罩，最终显示颜色由 WidgetryIcon 继承的前景色相乘得到。
fn star() -> impl Scene {
    bsn! {
        @WidgetryIcon {
            @path: { GalleryIcon::ButtonStar.path() },
            @max_size: { Some(UVec2::new(16, 16)) },
        }
    }
}

/// 刷新页面分组标题的继承色，不覆盖按钮自己的传播root。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<ButtonDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Plugin for ButtonDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_theme);
    }
}
