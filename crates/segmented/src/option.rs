use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetrySegmentedOptionProps)]
#[require(
    crate::group::OwnershipDiagnostics,
    crate::colors::OptionColorState,
    crate::style::StyleDiagnostics
)]
pub struct WidgetrySegmentedOption;

#[derive(Default, Clone, Debug)]
pub struct WidgetrySegmentedOptionProps {
    pub colors: crate::WidgetrySegmentedOptionColorOverrides,
}

impl WidgetrySegmentedOption {
    fn scene(props: WidgetrySegmentedOptionProps) -> impl Scene {
        bsn! {
            template(move |_| props.colors.clone().initial())
            bevy::ui::Checkable
            template(|_| Ok(bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::RadioButton))))
            bevy::picking::hover::Hovered(false)
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node {
                min_width: px(24), min_height: px(26),
                padding: UiRect::horizontal(px(10)),
                border_radius: BorderRadius::all(px(4)),
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            }
            BackgroundColor
            template(|_| Ok(bevy_widgetry_core::foreground::ResolvedForeground::default()))
        }
    }
}
