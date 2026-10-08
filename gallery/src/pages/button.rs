use crate::assets::GalleryIcon;
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};
use bevy_widgetry::{
    button::WidgetryButton,
    icon::WidgetryIcon,
    radio_group::{WidgetryRadioGroup, WidgetryRadioOption},
    style::ForegroundColor,
};

pub(crate) struct ButtonDemoPlugin;

#[derive(Component)]
struct ButtonDemo;

#[derive(Component)]
struct ButtonDemoAction(&'static str);

pub(crate) fn scene() -> impl Scene {
    bsn! {
        #ButtonDemo
        template(|_| Ok(ButtonDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<WidgetryThemeMode>().colors().text.normal.foreground))))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
        Children [
            Text("Button"),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Normal"), button_row(false)]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Disabled"), button_row(true)]),
            Text("Radio Button"),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Horizontal"), (
                    @WidgetryRadioGroup
                    Node { flex_direction: FlexDirection::Row, column_gap: px(16) }
                    on(on_radio_changed)
                    Children [radio_option("Apple"), radio_option("Banana"), radio_option("Orange")]
                )]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Grid"), (
                    @WidgetryRadioGroup
                    Node {
                        display: Display::Grid,
                        grid_template_columns: {vec![RepeatedGridTrack::auto(2)]},
                        column_gap: px(16),
                    }
                    on(on_radio_changed)
                    Children [radio_option("Apple"), radio_option("Banana"), radio_option("Orange"), radio_option("Grape")]
                )]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Text("Disabled"), (
                    @WidgetryRadioGroup InteractionDisabled
                    Children [radio_option("Apple"), radio_option("Banana"), radio_option("Orange")]
                )]),
        ]
    }
}

fn radio_option(label: &'static str) -> impl Scene {
    bsn! { @WidgetryRadioOption Children [Text(label)] }
}

fn on_radio_changed(event: On<ValueChange<usize>>) {
    info!(demo = "radio", entity = ?event.source, index = event.value, "选择 Radio 示例选项");
}

fn button_row(disabled: bool) -> impl Scene {
    let buttons = bsn_list![
        (@WidgetryButton {}
            template(|_| Ok(ButtonDemoAction("Text")))
            on(on_demo_activated)
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            Children [Text("Text")]),
        (@WidgetryButton {}
            template(|_| Ok(ButtonDemoAction("Icon + Text")))
            on(on_demo_activated)
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) }
            Children [star(), Text("Icon + Text")]),
        (@WidgetryButton {}
            template(|_| Ok(ButtonDemoAction("Text + Icon")))
            on(on_demo_activated)
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::Center, column_gap: px(6) }
            Children [Text("Text + Icon"), star()]),
        (@WidgetryButton {}
            template(|_| Ok(ButtonDemoAction("Icon")))
            on(on_demo_activated)
            {disabled.then(|| bsn! { InteractionDisabled })}
            Node { width: px(32), height: px(32), padding: UiRect::all(px(6)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
            Children [star()]),
    ];
    bsn! {
        Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(12) }
        Children [{buttons}]
    }
}

fn on_demo_activated(
    event: On<Activate>,
    buttons: Query<&ButtonDemoAction, Without<InteractionDisabled>>,
) {
    let Ok(action) = buttons.get(event.entity) else {
        return;
    };
    info!(demo = "button", button = action.0, "激活按钮示例");
}

fn star() -> impl Scene {
    bsn! {
        @WidgetryIcon {
            @path: { GalleryIcon::ButtonStar.path() },
            @max_size: { Some(UVec2::new(16, 16)) },
        }
    }
}

fn refresh_theme(
    event: On<WidgetryThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<ButtonDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().text.normal.foreground);
    }
}

impl Plugin for ButtonDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_theme);
    }
}
