use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::RadioButton;
use bevy_widgetry_core::foreground::ResolvedForeground;

#[derive(SceneComponent, Default, Clone)]
#[require(
    Node,
    RadioButton,
    Hovered,
    bevy_widgetry_core::pointer::WidgetryPointerPressed,
    crate::group::OwnershipDiagnostics
)]
pub struct WidgetryRadioOptionHeadless;

impl WidgetryRadioOptionHeadless {
    fn scene() -> impl Scene {
        bsn! {}
    }
}

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryRadioOptionProps)]
#[require(
    crate::colors::OptionColorState,
    crate::option_style::StyleDiagnostics,
    WidgetryRadioOptionHeadless
)]
pub struct WidgetryRadioOption;

#[derive(Default, Clone, Debug)]
pub struct WidgetryRadioOptionProps {
    pub colors: crate::WidgetryRadioOptionColorOverrides,
}

#[derive(Component)]
pub(crate) struct RadioIndicator;

#[derive(Component)]
pub(crate) struct RadioDot;

impl WidgetryRadioOption {
    fn scene(props: WidgetryRadioOptionProps) -> impl Scene {
        bsn! {
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            @WidgetryRadioOptionHeadless
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                min_height: px(24),
            }
            template(|_| Ok(ResolvedForeground::default()))
            Children [
                template(|_| Ok(RadioIndicator))
                Node {
                    width: px(16), height: px(16),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::MAX,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                BorderColor
                BackgroundColor
                Children [
                    template(|_| Ok(RadioDot))
                    Node { width: px(8), height: px(8), border_radius: BorderRadius::MAX }
                    BackgroundColor
                ]
            ]
        }
    }
}
