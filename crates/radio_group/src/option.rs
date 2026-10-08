use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::RadioButton;
use bevy_widgetry_core::ForegroundColor;

#[derive(SceneComponent, Default, Clone)]
#[require(
    crate::option_style::StyleDiagnostics,
    crate::group::OwnershipDiagnostics
)]
pub struct WidgetryRadioOption;

#[derive(Component)]
pub(crate) struct RadioIndicator;

#[derive(Component)]
pub(crate) struct RadioDot;

impl WidgetryRadioOption {
    fn scene() -> impl Scene {
        bsn! {
            RadioButton
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            Hovered(false)
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(6),
                min_height: px(24),
            }
            template(|_| Ok(Propagate(ForegroundColor::default())))
            Children [(
                template(|_| Ok(RadioIndicator))
                Node {
                    width: px(16), height: px(16),
                    border: UiRect::all(px(2)),
                    border_radius: BorderRadius::MAX,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                }
                BorderColor
                Children [(
                    template(|_| Ok(RadioDot))
                    Node { width: px(8), height: px(8), border_radius: BorderRadius::MAX }
                    BackgroundColor
                )]
            )]
        }
    }
}
