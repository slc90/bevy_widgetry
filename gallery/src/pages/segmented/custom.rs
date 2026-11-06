use super::{ProgrammaticTarget, SegmentedKey, demo, option};
use bevy::{
    input_focus::InputFocus,
    picking::hover::Hovered,
    prelude::*,
    ui::{Checked, InteractionDisabled},
};
use bevy_widgetry::segmented::{
    SegmentedOrientation, WidgetrySegmentedHeadless, WidgetrySegmentedOptionHeadless,
};
use bevy_widgetry::theme::WidgetryTheme;

#[derive(Component)]
struct ShapeChoice;
#[derive(Clone, Copy)]
enum Shape {
    Pill,
    Square,
    Rounded,
}
#[derive(Component)]
struct CustomGroup;

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { column_gap: px(24), row_gap: px(20), flex_wrap: FlexWrap::Wrap, align_items: AlignItems::Start }
        Children [
            @demo(SegmentedKey::CustomShapes, "SegmentedCustom", bsn! {
                @WidgetrySegmentedHeadless template(|_|Ok(CustomGroup)) template(|_|Ok(ProgrammaticTarget))
                Node { column_gap: px(8), border: UiRect::all(px(1)), padding: UiRect::all(px(6)), border_radius: BorderRadius::all(px(8)) }
                BorderColor BackgroundColor({Color::srgb_u8(20,35,48)})
                Children [
                    @shape_option("SegmentedCustom0", Shape::Pill)--
                    @shape_option("SegmentedCustom1", Shape::Square) InteractionDisabled--
                    @shape_option("SegmentedCustom2", Shape::Rounded)
                ]
            },0)--
            @demo(SegmentedKey::Mixed, "SegmentedMixed", bsn! {
                @WidgetrySegmentedHeadless { @orientation: SegmentedOrientation::Vertical }
                template(|_|Ok(CustomGroup)) template(|_|Ok(ProgrammaticTarget))
                Node { flex_direction: FlexDirection::Column, row_gap: px(8), border: UiRect::all(px(1)), padding: UiRect::all(px(6)), border_radius: BorderRadius::all(px(8)), align_items: AlignItems::Stretch }
                BorderColor BackgroundColor({Color::srgb_u8(20,35,48)})
                Children [
                    @option("SegmentedMixed0", SegmentedKey::DefaultChoice)--
                    @shape_option("SegmentedMixed1", Shape::Square)--
                    @shape_option("SegmentedMixed2", Shape::Rounded)
                ]
            },0)
        ]
    }
}

fn shape_option(name: &'static str, shape: Shape) -> impl Scene {
    let radius = match shape {
        Shape::Pill => 21,
        Shape::Square => 3,
        Shape::Rounded => 10,
    };
    bsn! {
        @WidgetrySegmentedOptionHeadless Name(name) template(|_|Ok(ShapeChoice))
        template(|_|Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
        Node { width: px(94), height: px(42), border: UiRect::all(px(2)), border_radius: BorderRadius::all(px(radius)), align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        BackgroundColor BorderColor

    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        visuals
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::Segmented)),
    );
}

fn visuals(
    focus: Res<InputFocus>,
    theme: Res<WidgetryTheme>,
    mut roots: Query<(Entity, &mut BorderColor, &mut BackgroundColor), With<CustomGroup>>,
    mut options: Query<
        (
            Has<Checked>,
            Has<InteractionDisabled>,
            &Hovered,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (With<ShapeChoice>, Without<CustomGroup>),
    >,
) {
    for (root, mut border, mut background) in &mut roots {
        background.set_if_neq(BackgroundColor(theme.segmented.container.normal.background));
        border.set_if_neq(BorderColor::all(if focus.get() == Some(root) {
            Color::srgb_u8(55, 205, 175)
        } else {
            Color::srgb_u8(65, 90, 110)
        }));
    }
    for (selected, disabled, hovered, mut border, mut background) in &mut options {
        let color = if disabled {
            Color::srgb_u8(110, 125, 135)
        } else if selected {
            Color::srgb_u8(55, 205, 175)
        } else {
            Color::srgb_u8(225, 235, 240)
        };
        border.set_if_neq(BorderColor::all(color));
        background.set_if_neq(BackgroundColor(if selected {
            Color::srgb_u8(20, 95, 85)
        } else if hovered.0 && !disabled {
            Color::srgb_u8(35, 65, 80)
        } else {
            Color::srgb_u8(20, 35, 48)
        }));
    }
}
