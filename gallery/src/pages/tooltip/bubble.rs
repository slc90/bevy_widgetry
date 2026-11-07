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
        Node {flex_direction:FlexDirection::Column,row_gap:px(8),align_items:AlignItems::Start}
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
        let left=center.x+node.size().x*node.inverse_scale_factor()/2.0+12.0;
        let top=(center.y-24.0).max(8.0);
        let colors=world.resource::<WidgetryTheme>().tooltip.popup.normal;
        let accent=world.resource::<WidgetryTheme>().progress_bar.fill;
        let popup=spawn_scene(world,bsn!{
            Bubble Name::new("TooltipBubblePopup") UiTargetCamera(camera) GlobalZIndex(10000) Pickable::IGNORE
            Node {position_type:PositionType::Absolute,left:px(left),top:px(top),max_width:px(280),padding:UiRect::all(px(14)),border:UiRect::all(px(2)),border_radius:BorderRadius::all(px(18)),column_gap:px(10),align_items:AlignItems::Center}
            BackgroundColor({colors.background}) BorderColor::all(colors.border)
            Children [
                Node {width:px(8),height:px(32),flex_shrink:0.0,border_radius:BorderRadius::MAX} BackgroundColor({accent}) Pickable::IGNORE--
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
