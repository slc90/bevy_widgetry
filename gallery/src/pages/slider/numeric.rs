const EDITOR_WIDTH: f32 = 480.0;
const EDITOR_MIN_HEIGHT: f32 = 88.0;
const EDITOR_PADDING: f32 = 12.0;
const EDITOR_BORDER_WIDTH: f32 = 2.0;
const EDITOR_CORNER_RADIUS: f32 = 16.0;

const DETAIL_GAP: f32 = 8.0;
const CONTROL_GAP: f32 = 8.0;
const LABEL_FONT_SIZE: f32 = 24.0;

use crate::i18n::{Language, SliderKey, TextKey, localized, tr_args};
use bevy::{
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ValueChange},
};
use bevy_widgetry::{
    button::WidgetryButton,
    slider::{WidgetrySliderHeadless, WidgetrySliderState},
    text::WidgetryText,
    theme::WidgetryTheme,
};

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct NumericReadout {
    changes: usize,
}
#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Set(f32),
    Disabled,
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Name("SliderNumericDemo")
        Node { width:percent(100), flex_direction:FlexDirection::Column, row_gap:px(DETAIL_GAP), align_items:AlignItems::Start, flex_shrink:0.0 }
        Children [
            @localized(TextKey::Slider(SliderKey::NumericDescription)) WidgetryText--
            @WidgetrySliderHeadless { @max:10.0, @step:{Some(3.0)}, @value:{Some(4.0)} }
            Name("SliderNumericReadout") template(|_|Ok(NumericReadout::default())) on(changed)
            Node { width:px(EDITOR_WIDTH), max_width:percent(100), min_height:px(EDITOR_MIN_HEIGHT), padding:UiRect::all(px(EDITOR_PADDING)), border:UiRect::all(px(EDITOR_BORDER_WIDTH)), border_radius:BorderRadius::all(px(EDITOR_CORNER_RADIUS)), align_items:AlignItems::Center }
            BackgroundColor BorderColor Text("") WidgetryText TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}--
            Node { column_gap:px(CONTROL_GAP), row_gap:px(DETAIL_GAP), flex_wrap:FlexWrap::Wrap }
            Children [
                @control(Action::Set(0.0),SliderKey::NumericMin,"SliderNumericMin")--
                @control(Action::Set(4.0),SliderKey::NumericFour,"SliderNumericFour")--
                @control(Action::Set(10.0),SliderKey::NumericMax,"SliderNumericMax")--
                @control(Action::Disabled,SliderKey::NumericDisabled,"SliderNumericDisabled")
            ]
        ]
    }
}

fn control(action: Action, key: SliderKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children:{Some(Box::new(bsn_list! { @localized(TextKey::Slider(key)) WidgetryText }) as Box<dyn SceneList>)} }
        Name(name) template(move |_|Ok(action)) on(operate)
    }
}

fn changed(event: On<ValueChange<f32>>, mut readouts: Query<&mut NumericReadout>) {
    if let Ok(mut readout) = readouts.get_mut(event.source) {
        readout.changes += 1;
        info!(entity=?event.source,value=event.value,is_final=event.is_final,changes=readout.changes,"独立数值读数已提交");
    }
}
fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    readouts: Query<(Entity, Has<InteractionDisabled>), With<NumericReadout>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    for (root, disabled) in &readouts {
        match *action {
            Action::Set(value) => WidgetrySliderHeadless::set_value(&mut commands, root, value),
            Action::Disabled => {
                if disabled {
                    commands.entity(root).remove::<InteractionDisabled>();
                } else {
                    commands.entity(root).insert(InteractionDisabled);
                }
            }
        }
        info!(?root, ?action, "操作独立数值读数");
    }
}
pub(super) fn install(app: &mut App) {
    app.register_type::<NumericReadout>().add_systems(
        PostUpdate,
        update
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::Slider)),
    );
}
fn update(
    language: Res<Language>,
    theme: Res<WidgetryTheme>,
    mut readouts: Query<(
        Ref<WidgetrySliderState>,
        Ref<NumericReadout>,
        Has<InteractionDisabled>,
        &mut Text,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (state, readout, disabled, mut text, mut background, mut border) in &mut readouts {
        if state.is_changed() || readout.is_changed() || language.is_changed() {
            text.set_if_neq(Text(tr_args(
                *language,
                TextKey::Slider(SliderKey::NumericStatus),
                &[
                    ("value", format!("{:.2}", state.value())),
                    ("min", state.min().to_string()),
                    ("max", state.max().to_string()),
                    (
                        "step",
                        state
                            .step()
                            .map_or_else(|| "-".to_owned(), |step| step.to_string()),
                    ),
                    ("changes", readout.changes.to_string()),
                ],
            )));
        }
        background.set_if_neq(BackgroundColor(theme.segmented.container.normal.background));
        border.set_if_neq(BorderColor::all(if disabled {
            Color::srgb_u8(105, 115, 125)
        } else {
            Color::srgb_u8(55, 205, 175)
        }));
    }
}
