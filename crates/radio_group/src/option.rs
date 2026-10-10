use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::RadioButton;
use bevy_widgetry_core::foreground::ResolvedForeground;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryRadioOptionProps)]
#[require(
    crate::colors::OptionColorState,
    crate::option_style::StyleDiagnostics,
    crate::group::OwnershipDiagnostics
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
            template(move |_| props.colors.clone().initial())
            RadioButton
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            Hovered(false)
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
