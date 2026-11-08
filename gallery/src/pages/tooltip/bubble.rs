const BUBBLE_GAP: f32 = 12.0;
const BUBBLE_VERTICAL_OFFSET: f32 = 24.0;
const BUBBLE_WINDOW_MARGIN: f32 = 8.0;
const BUBBLE_Z_INDEX: i32 = 10000;

const BUBBLE_MAX_WIDTH: f32 = 280.0;
const BUBBLE_PADDING: f32 = 14.0;
const BUBBLE_BORDER_WIDTH: f32 = 2.0;
const BUBBLE_CORNER_RADIUS: f32 = 18.0;
const ACCENT_WIDTH: f32 = 8.0;
const ACCENT_HEIGHT: f32 = 32.0;

const DETAIL_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 10.0;

use crate::i18n::{TextKey, TooltipKey, localized};
use bevy::{
    prelude::*,
    ui::{ComputedUiTargetCamera, UiGlobalTransform},
};
use bevy_widgetry::{
    button::WidgetryButton,
    scene::spawn_scene,
    text::{WidgetryText, WidgetryTextColorOverrides, WidgetryTextStateColorOverrides},
    theme::WidgetryTheme,
    tooltip::{ShowTooltip, WidgetryTooltipHeadless},
};

#[derive(Component, Default, Clone)]
struct Anchor;
#[derive(Component, Default, Clone)]
struct Bubble;

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(DETAIL_GAP),align_items:AlignItems::Start}
        Children [
            @localized(TextKey::Tooltip(TooltipKey::CustomBubble)) WidgetryText--
            Anchor Name::new("TooltipBubbleAnchor") @WidgetryTooltipHeadless
            @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::Tooltip(TooltipKey::HoverBubble)) WidgetryText}) as Box<dyn SceneList>)}}
        ]
    }
}

pub(super) fn install(app: &mut App) {
    app.add_observer(show).add_systems(
        PostUpdate,
        refresh_colors.run_if(resource_changed::<WidgetryTheme>),
    );
}

fn show(event: On<ShowTooltip>, anchors: Query<(), With<Anchor>>, mut commands: Commands) {
    if !anchors.contains(event.source) {
        return;
    }
    let show = *event;
    commands.queue(move |world: &mut World| -> Result {
        if !show.is_current(world) || show.popup(world).is_some() { return Ok(()); }
        let (Some(transform),Some(node),Some(camera))=(world.get::<UiGlobalTransform>(show.source),world.get::<ComputedNode>(show.source),world.get::<ComputedUiTargetCamera>(show.source).and_then(|target|target.get())) else { return Ok(()); };
        let center=transform.transform_point2(Vec2::ZERO)*node.inverse_scale_factor();
        let left=center.x+node.size().x*node.inverse_scale_factor()/2.0+BUBBLE_GAP;
        let top=(center.y-BUBBLE_VERTICAL_OFFSET).max(BUBBLE_WINDOW_MARGIN);
        let colors=world.resource::<WidgetryTheme>().tooltip.popup.normal;
        let accent=world.resource::<WidgetryTheme>().progress_bar.fill;
        let popup=spawn_scene(world,bsn!{
            Bubble Name::new("TooltipBubblePopup") UiTargetCamera(camera) GlobalZIndex(BUBBLE_Z_INDEX) Pickable::IGNORE
            Node {position_type:PositionType::Absolute,left:px(left),top:px(top),max_width:px(BUBBLE_MAX_WIDTH),padding:UiRect::all(px(BUBBLE_PADDING)),border:UiRect::all(px(BUBBLE_BORDER_WIDTH)),border_radius:BorderRadius::all(px(BUBBLE_CORNER_RADIUS)),column_gap:px(CONTROL_GAP),align_items:AlignItems::Center}
            BackgroundColor({colors.background}) BorderColor::all(colors.border)
            Children [
                Node {width:px(ACCENT_WIDTH),height:px(ACCENT_HEIGHT),flex_shrink:0.0,border_radius:BorderRadius::MAX} BackgroundColor({accent}) Pickable::IGNORE--
                @localized(TextKey::Tooltip(TooltipKey::BubbleContent))
                @WidgetryText {@colors:{WidgetryTextColorOverrides {normal:WidgetryTextStateColorOverrides {foreground:Some(colors.foreground)},disabled:WidgetryTextStateColorOverrides {foreground:Some(colors.foreground)}}}} Pickable::IGNORE
            ]
        }).map_err(|error|error.into_bevy_error())?;
        show.register_popup(world,popup).map_err(|error|error.into_bevy_error())?;
        Ok(())
    });
}

fn refresh_colors(
    theme: Res<WidgetryTheme>,
    mut bubbles: Query<(&mut BackgroundColor, &mut BorderColor, &Children), With<Bubble>>,
    mut backgrounds: Query<&mut BackgroundColor, Without<Bubble>>,
    text: Query<(), With<WidgetryText>>,
    mut commands: Commands,
) {
    let colors = theme.tooltip.popup.normal;
    for (mut background, mut border, children) in &mut bubbles {
        background.set_if_neq(BackgroundColor(colors.background));
        border.set_if_neq(BorderColor::all(colors.border));
        for child in children {
            if let Ok(mut background) = backgrounds.get_mut(*child) {
                background.set_if_neq(BackgroundColor(theme.progress_bar.fill));
            }
            if text.contains(*child) {
                WidgetryText::set_color(&mut commands, *child, colors.foreground);
            }
        }
    }
}
