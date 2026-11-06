use crate::indicator::checkbox_indicator_scene;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor};
use bevy::ui_widgets::{Checkbox, checkbox_self_update};
use bevy_widgetry_core::foreground::ResolvedForeground;

#[derive(SceneComponent, Default, Clone)]
#[require(Node, Checkbox, Hovered, bevy_widgetry_core::pointer::WidgetryPointerPressed, TabIndex(-1))]
pub struct WidgetryCheckBoxHeadless;

impl WidgetryCheckBoxHeadless {
    fn scene() -> impl Scene {
        bsn! {}
    }
}

pub struct WidgetryCheckBoxHeadlessPlugin;

impl Plugin for WidgetryCheckBoxHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::CheckboxPlugin>() {
            app.add_plugins(bevy::ui_widgets::CheckboxPlugin);
        }
        app.add_observer(self_update);
        bevy_widgetry_internal_log::widgetry_debug!("WidgetryCheckBoxHeadlessPlugin 注册完成");
    }
}

fn self_update(
    event: On<bevy::ui_widgets::ValueChange<bool>>,
    roots: Query<(), With<WidgetryCheckBoxHeadless>>,
    commands: Commands,
) {
    if roots.contains(event.source) {
        checkbox_self_update(event, commands);
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryCheckBoxProps)]
#[require(
    WidgetryCheckBoxHeadless,
    crate::style::StyleDiagnostics,
    crate::colors::ColorState
)]
pub struct WidgetryCheckBox;

impl WidgetryCheckBox {
    fn scene(props: crate::WidgetryCheckBoxProps) -> impl Scene {
        bsn! {
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            @WidgetryCheckBoxHeadless
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), min_height: px(24) }
            BackgroundColor
            BorderColor
            template(|_| Ok(ResolvedForeground::default()))
            Children [@checkbox_indicator_scene()]
        }
    }
}
