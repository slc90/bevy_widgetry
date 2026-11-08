const EDITOR_WIDTH: f32 = 300.0;
const EDITOR_PADDING: f32 = 12.0;
const EDITOR_CORNER_RADIUS: f32 = 12.0;
const EDITOR_UNDERLINE_WIDTH: f32 = 3.0;
const EDITOR_MIN_HEIGHT: f32 = 28.0;
const STEP_COLUMN_WIDTH: f32 = 36.0;
const STEP_BUTTON_HEIGHT: f32 = 24.0;
const STEP_BUTTON_BORDER_WIDTH: f32 = 1.0;
const STEP_BUTTON_CORNER_RADIUS: f32 = 4.0;

const DETAIL_GAP: f32 = 4.0;
const ITEM_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 8.0;
const LABEL_FONT_SIZE: f32 = 20.0;

use crate::i18n::{Language, NumberInputKey, TextKey, localized, tr_args};
use bevy::prelude::*;
use bevy::text::TextCursorStyle;
use bevy_widgetry::{
    number_input::{
        WidgetryNumberInputEditor, WidgetryNumberInputHeadless, WidgetryNumberInputRepeat,
        WidgetryNumberInputStep,
    },
    text::WidgetryText,
    text_field::{WidgetryTextFieldEditor, WidgetryTextFieldHeadless},
    theme::WidgetryTheme,
};

#[derive(Component, FromTemplate)]
struct Status {
    root: Entity,
}

#[derive(Component)]
struct StepAppearance;

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(ITEM_GAP)}
        Children [
            @localized(TextKey::NumberInput(NumberInputKey::CustomEditing)) WidgetryText--
            #Root @WidgetryNumberInputHeadless<f64> {@value:{Some(12.345)},@decimals:{Some(2)}}
            WidgetryNumberInputEditor {field:#Field} Name::new("NumberCustom")
            on(committed)
            Node {width:px(EDITOR_WIDTH),padding:UiRect::all(px(EDITOR_PADDING)),border:UiRect::bottom(px(EDITOR_UNDERLINE_WIDTH)),border_radius:BorderRadius::all(px(EDITOR_CORNER_RADIUS))}
            BackgroundColor BorderColor
            Children [
                #Field @WidgetryTextFieldHeadless {editor:#Editor}
                Node {width:percent(100)}
                Children [Node {width:percent(100)} Children [
                    #Editor @WidgetryTextFieldEditor {root:#Field}
                    Name::new("NumberCustomEditor") Node {width:percent(100),min_height:px(EDITOR_MIN_HEIGHT)}
                    TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}
                ]]
            ]--
            Status {root:#Root} Name::new("NumberCustomStatus") Text::new("") WidgetryText--
            @stepped_scene(false)--
            @stepped_scene(true)
        ]
    }
}

fn stepped_scene(repeat: bool) -> impl Scene {
    let repeat_up = repeat.then_some(bsn! {WidgetryNumberInputRepeat});
    let repeat_down = repeat.then_some(bsn! {WidgetryNumberInputRepeat});
    bsn! {
        Node {flex_direction:FlexDirection::Column,row_gap:px(ITEM_GAP)}
        Children [
            @localized(TextKey::NumberInput(if repeat {NumberInputKey::CustomRepeat} else {NumberInputKey::CustomStepping})) WidgetryText--
            #Root @WidgetryNumberInputHeadless<f64> {@value:{Some(10.0)},@min:{Some(0.0)},@max:{Some(100.0)},@step:5.0}
            WidgetryNumberInputEditor {field:#Field} Name::new(if repeat {"NumberCustomRepeat"} else {"NumberCustomStep"}) on(committed)
            Node {width:px(EDITOR_WIDTH),padding:UiRect::all(px(EDITOR_PADDING)),column_gap:px(CONTROL_GAP),align_items:AlignItems::Center,border:UiRect::bottom(px(EDITOR_UNDERLINE_WIDTH)),border_radius:BorderRadius::all(px(EDITOR_CORNER_RADIUS))}
            BackgroundColor BorderColor
            Children [
                #Field @WidgetryTextFieldHeadless {editor:#Editor}
                Node {flex_grow:1.0,min_width:px(0)}
                Children [Node {width:percent(100)} Children [
                    #Editor @WidgetryTextFieldEditor {root:#Field}
                    Name::new(if repeat {"NumberCustomRepeatEditor"} else {"NumberCustomStepEditor"}) Node {width:percent(100),min_height:px(EDITOR_MIN_HEIGHT)} TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}
                ]]--
                Node {flex_direction:FlexDirection::Column,row_gap:px(DETAIL_GAP),width:px(STEP_COLUMN_WIDTH),flex_shrink:0.0}
                Children [
                    WidgetryNumberInputStep {root:#Root,increase:true} @{repeat_up} Name::new(if repeat {"NumberCustomRepeatIncrease"} else {"NumberCustomIncrease"})
                    template(|_|Ok(StepAppearance))
                    bevy::ui_widgets::Button bevy::ui_widgets::ActivateOnPress
                    bevy_widgetry_core::pointer::WidgetryPointerPressed bevy::picking::hover::Hovered(false)
                    Node {width:percent(100),height:px(STEP_BUTTON_HEIGHT),justify_content:JustifyContent::Center,align_items:AlignItems::Center,border:UiRect::all(px(STEP_BUTTON_BORDER_WIDTH)),border_radius:BorderRadius::all(px(STEP_BUTTON_CORNER_RADIUS))} BorderColor BackgroundColor
                    Children [Text::new("+") WidgetryText Pickable::IGNORE]--
                    WidgetryNumberInputStep {root:#Root,increase:false} @{repeat_down} Name::new(if repeat {"NumberCustomRepeatDecrease"} else {"NumberCustomDecrease"})
                    template(|_|Ok(StepAppearance))
                    bevy::ui_widgets::Button bevy::ui_widgets::ActivateOnPress
                    bevy_widgetry_core::pointer::WidgetryPointerPressed bevy::picking::hover::Hovered(false)
                    Node {width:percent(100),height:px(STEP_BUTTON_HEIGHT),justify_content:JustifyContent::Center,align_items:AlignItems::Center,border:UiRect::all(px(STEP_BUTTON_BORDER_WIDTH)),border_radius:BorderRadius::all(px(STEP_BUTTON_CORNER_RADIUS))} BorderColor BackgroundColor
                    Children [Text::new("−") WidgetryText Pickable::IGNORE]
                ]
            ]--
            Status {root:#Root} Name::new(if repeat {"NumberCustomRepeatStatus"} else {"NumberCustomStepStatus"}) Text::new("") WidgetryText
        ]
    }
}

fn committed(event: On<bevy::ui_widgets::ValueChange<f64>>) {
    info!(source=?event.source,value=event.value,"自定义数字框提交正式值");
}

pub(super) fn refresh(world: &mut World) -> Result {
    let theme = *world.resource::<WidgetryTheme>();
    let mut changed = false;
    let labels: Vec<_> = world
        .query::<(Entity, &Status)>()
        .iter(world)
        .map(|(label, status)| (label, status.root))
        .collect();
    for (label, root) in labels {
        let value = WidgetryNumberInputHeadless::<f64>::value(world, root)
            .map_err(|error| error.into_bevy_error())?;
        let field = world
            .get::<WidgetryNumberInputEditor>(root)
            .ok_or_else(|| crate::diagnostics::error("自定义数字框缺失编辑绑定"))?
            .field;
        let draft = WidgetryTextFieldHeadless::text(world, field)
            .map_err(|error| error.into_bevy_error())?;
        let text = tr_args(
            *world.resource::<Language>(),
            TextKey::NumberInput(NumberInputKey::CustomDraftStatus),
            &[("value", value.to_string()), ("draft", draft)],
        );
        if let Some(mut current) = world.get_mut::<Text>(label) {
            changed |= current.set_if_neq(Text(text));
        }
        let editor = WidgetryNumberInputHeadless::<f64>::editor_entity(world, root)
            .map_err(|error| error.into_bevy_error())?;
        let colors = theme.text_field.editable.normal;
        if let Some(mut color) = world.get_mut::<TextColor>(editor) {
            color.set_if_neq(TextColor(colors.foreground));
        }
        if let Some(mut caret) = world.get_mut::<TextCursorStyle>(editor) {
            if caret.color != colors.caret {
                caret.color = colors.caret;
            }
            if caret.selection_color != colors.selection_background {
                caret.selection_color = colors.selection_background;
            }
            if caret.selected_text_color != Some(colors.selection_foreground) {
                caret.selected_text_color = Some(colors.selection_foreground);
            }
        }
        if let Some(mut background) = world.get_mut::<BackgroundColor>(root) {
            background.set_if_neq(BackgroundColor(theme.text_field.editable.normal.background));
        }
        if let Some(mut border) = world.get_mut::<BorderColor>(root) {
            border.set_if_neq(BorderColor::all(theme.text_field.editable.focused.border));
        }
    }
    for (hovered, pressed, mut background, mut border) in world
        .query_filtered::<(
            &bevy::picking::hover::Hovered,
            Has<bevy::ui::Pressed>,
            &mut BackgroundColor,
            &mut BorderColor,
        ), With<StepAppearance>>()
        .iter_mut(world)
    {
        let colors = if pressed {
            theme.button.pressed
        } else if hovered.0 {
            theme.button.hovered
        } else {
            theme.button.normal
        };
        background.set_if_neq(BackgroundColor(colors.background));
        border.set_if_neq(BorderColor::all(theme.text_field.editable.focused.border));
    }
    if changed {
        world.write_message(bevy::window::RequestRedraw);
    }
    Ok(())
}
