use bevy::prelude::*;

#[derive(SceneComponent, Default, Clone)]
#[require(
    Node,
    bevy::ui::Checkable,
    bevy::picking::hover::Hovered,
    bevy_widgetry_core::pointer::WidgetryPointerPressed,
    crate::group::OwnershipDiagnostics
)]
pub struct WidgetrySegmentedOptionHeadless;
impl WidgetrySegmentedOptionHeadless {
    fn scene() -> impl Scene {
        bsn! { template(|_| Ok(bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::RadioButton)))) }
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetrySegmentedOptionProps)]
#[require(
    WidgetrySegmentedOptionHeadless,
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
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            @WidgetrySegmentedOptionHeadless
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
