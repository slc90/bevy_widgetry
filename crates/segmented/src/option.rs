const OPTION_MIN_WIDTH: f32 = 24.0;
const OPTION_MIN_HEIGHT: f32 = 26.0;
const OPTION_PADDING_X: f32 = 10.0;
const OPTION_CORNER_RADIUS: f32 = 4.0;

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
        bsn! {}
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
                min_width: px(OPTION_MIN_WIDTH), min_height: px(OPTION_MIN_HEIGHT),
                padding: UiRect::horizontal(px(OPTION_PADDING_X)),
                border_radius: BorderRadius::all(px(OPTION_CORNER_RADIUS)),
                align_items: AlignItems::Center, justify_content: JustifyContent::Center,
            }
            BackgroundColor
            template(|_| Ok(bevy_widgetry_core::foreground::ResolvedForeground::default()))
        }
    }
}
