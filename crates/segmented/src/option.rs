use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[require(crate::group::OwnershipDiagnostics)]
pub struct WidgetrySegmentedOption;

impl WidgetrySegmentedOption {
    fn scene() -> impl Scene {
        bsn! {
            bevy::ui::Checkable
            template(|_| Ok(bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::RadioButton))))
            bevy::picking::hover::Hovered(false)
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { min_width: px(24), min_height: px(24), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        }
    }
}
