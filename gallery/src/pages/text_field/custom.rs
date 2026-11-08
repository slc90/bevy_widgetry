const MAGNIFIER_HANDLE_ANGLE: f32 = -35.0;
const EXAMPLE_BOTTOM_MARGIN: f32 = 20.0;
const EDITOR_WIDTH: f32 = 480.0;
const SEARCH_MIN_HEIGHT: f32 = 50.0;
const SEARCH_PADDING: f32 = 10.0;
const SEARCH_UNDERLINE_WIDTH: f32 = 3.0;
const SEARCH_CORNER_RADIUS: f32 = 12.0;
const MAGNIFIER_DIAMETER: f32 = 18.0;
const MAGNIFIER_BORDER_WIDTH: f32 = 2.0;
const MAGNIFIER_HANDLE_LEFT: f32 = 13.0;
const MAGNIFIER_HANDLE_TOP: f32 = 11.0;
const MAGNIFIER_HANDLE_WIDTH: f32 = 3.0;
const MAGNIFIER_HANDLE_HEIGHT: f32 = 10.0;

const DETAIL_GAP: f32 = 6.0;
const ITEM_GAP: f32 = 8.0;
const GROUP_GAP: f32 = 12.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 12.0;
const LABEL_FONT_SIZE: f32 = 14.0;
const TITLE_FONT_SIZE: f32 = 18.0;

use crate::i18n::{Language, TextFieldKey, TextKey, localized, tr_args};
use bevy::{
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::{InteractionDisabled, UiTransform},
    ui_widgets::Activate,
};
use bevy_widgetry::{
    button::WidgetryButton,
    text::WidgetryText,
    text_field::{WidgetryTextFieldEditor, WidgetryTextFieldHeadless},
    theme::WidgetryTheme,
};

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Editable,
    ReadOnly,
}
#[derive(Component)]
struct SearchPanel;
#[derive(Component, FromTemplate)]
struct ContentLabel(Entity);
#[derive(Clone, Copy, Debug)]
enum Operation {
    Clear,
    Focus,
    Preset,
    Disabled,
}
#[derive(Component, Clone, Copy, Debug)]
struct Action {
    mode: Mode,
    operation: Operation,
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(GROUP_GAP),margin:UiRect::bottom(px(EXAMPLE_BOTTOM_MARGIN))}
        Children [
            @localized(TextKey::TextField(TextFieldKey::CustomDescription)) WidgetryText--
            @field(Mode::Editable)--
            @controls(Mode::Editable)--
            @field(Mode::ReadOnly)--
            @controls(Mode::ReadOnly)
        ]
    }
}
fn field(mode: Mode) -> impl Scene {
    let read_only = mode == Mode::ReadOnly;
    let name = if read_only {
        "TextFieldCustomReadOnly"
    } else {
        "TextFieldCustomSearch"
    };
    let editor_name = if read_only {
        "TextFieldCustomReadOnlyEditor"
    } else {
        "TextFieldCustomSearchEditor"
    };
    let value = if read_only {
        "Select and copy"
    } else {
        "widget"
    }
    .to_owned();
    bsn! {
        #Root @WidgetryTextFieldHeadless {editor:#Editor,@read_only:read_only}
        template(move |_|Ok(mode)) Name(name)
        bevy::input_focus::tab_navigation::TabIndex(0)
        Node {width:px(EDITOR_WIDTH),max_width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(DETAIL_GAP),flex_shrink:0.0}
        Children [
            template(|_|Ok(SearchPanel)) Node {width:percent(100),min_height:px(SEARCH_MIN_HEIGHT),padding:UiRect::all(px(SEARCH_PADDING)),column_gap:px(CONTENT_GAP),align_items:AlignItems::Center,border:UiRect::bottom(px(SEARCH_UNDERLINE_WIDTH)),border_radius:BorderRadius::all(px(SEARCH_CORNER_RADIUS))}
            BackgroundColor BorderColor
            Children [
                Pickable::IGNORE Node {width:px(MAGNIFIER_DIAMETER),height:px(MAGNIFIER_DIAMETER),border:UiRect::all(px(MAGNIFIER_BORDER_WIDTH)),border_radius:BorderRadius::MAX,flex_shrink:0.0} ~{BorderColor::all(Color::srgb_u8(40,200,180))}
                Children [
                    Pickable::IGNORE Node {position_type:PositionType::Absolute,width:px(MAGNIFIER_HANDLE_WIDTH),height:px(MAGNIFIER_HANDLE_HEIGHT),left:px(MAGNIFIER_HANDLE_LEFT),top:px(MAGNIFIER_HANDLE_TOP)} BackgroundColor({Color::srgb_u8(40,200,180)})
                    UiTransform {rotation:Rot2::degrees(MAGNIFIER_HANDLE_ANGLE)}
                ]--
                Node {flex_grow:1.0,min_width:px(0)}
                Children [
                    Node {width:percent(100),align_items:AlignItems::Center}
                    Children [
                        #Editor @WidgetryTextFieldEditor {root:#Root,@value:value,@max_length:{Some(16)}}
                        Name(editor_name) TextFont {font_size:FontSize::Px(TITLE_FONT_SIZE)}
                        Node {width:percent(100),min_width:px(0)}
                    ]
                ]--
                @control(mode,Operation::Clear,TextFieldKey::CustomClear,if read_only {"TextFieldCustomReadOnlyClear"} else {"TextFieldCustomClear"})
            ]--
            @localized(TextKey::TextField(if read_only {TextFieldKey::CustomReadOnlyHint} else {TextFieldKey::CustomHint})) WidgetryText
            TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}--
            ContentLabel(#Root) Text("") WidgetryText TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}
        ]
    }
}
fn controls(mode: Mode) -> impl Scene {
    let readonly = mode == Mode::ReadOnly;
    bsn! {
        Node {column_gap:px(CONTROL_GAP),row_gap:px(ITEM_GAP),flex_wrap:FlexWrap::Wrap}
        Children [
            @control(mode,Operation::Focus,TextFieldKey::CustomFocus,if readonly {"TextFieldCustomReadOnlyFocus"} else {"TextFieldCustomFocus"})--
            @control(mode,Operation::Preset,TextFieldKey::CustomPreset,if readonly {"TextFieldCustomReadOnlyPreset"} else {"TextFieldCustomPreset"})--
            @control(mode,Operation::Disabled,TextFieldKey::CustomDisabled,if readonly {"TextFieldCustomReadOnlyDisabled"} else {"TextFieldCustomDisabled"})
        ]
    }
}
fn control(mode: Mode, operation: Operation, key: TextFieldKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton { @children:{Some(Box::new(bsn_list!{@localized(TextKey::TextField(key)) WidgetryText TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}}) as Box<dyn SceneList>)} }
        template(move |_|Ok(Action {mode,operation})) Name(name) on(operate)
    }
}
fn operate(
    event: On<Activate>,
    actions: Query<&Action>,
    roots: Query<(Entity, &Mode, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity).copied() else {
        return;
    };
    for (root, mode, disabled) in &roots {
        if *mode != action.mode {
            continue;
        }
        commands.queue(move |world: &mut World| -> Result {
            let changed = match action.operation {
                Operation::Clear => Some(
                    WidgetryTextFieldHeadless::set_text_in_world(world, root, "")
                        .map_err(|error| error.into_bevy_error())?,
                ),
                Operation::Preset => Some(
                    WidgetryTextFieldHeadless::set_text_in_world(world, root, "e\u{301}👍🏽👨‍👩‍👧‍👦")
                        .map_err(|error| error.into_bevy_error())?,
                ),
                Operation::Focus => {
                    WidgetryTextFieldHeadless::focus_in_world(world, root)
                        .map_err(|error| error.into_bevy_error())?;
                    None
                }
                Operation::Disabled => {
                    if disabled {
                        world.entity_mut(root).remove::<InteractionDisabled>();
                    } else {
                        world.entity_mut(root).insert(InteractionDisabled);
                    }
                    None
                }
            };
            info!(?root, ?action, ?changed, "操作自定义编辑区");
            Ok(())
        });
    }
}
pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        update
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::TextField)),
    );
}
fn update(
    language: Res<Language>,
    theme: Res<WidgetryTheme>,
    roots: Query<(&WidgetryTextFieldHeadless, Has<InteractionDisabled>, &Mode)>,
    mut panels: Query<(&ChildOf, &mut BackgroundColor, &mut BorderColor), With<SearchPanel>>,
    mut editors: Query<(&EditableText, &mut TextColor, &mut TextCursorStyle)>,
    mut labels: Query<(&ContentLabel, &mut Text)>,
) -> Result {
    for (parent, mut background, mut border) in &mut panels {
        let (binding, disabled, mode) = roots
            .get(parent.parent())
            .map_err(crate::diagnostics::error)?;
        let (edit, mut text, mut caret) = editors
            .get_mut(binding.editor)
            .map_err(crate::diagnostics::error)?;
        let colors = if *mode == Mode::ReadOnly {
            theme.text_field.read_only
        } else {
            theme.text_field.editable
        };
        let colors = if disabled {
            colors.disabled
        } else {
            colors.normal
        };
        text.set_if_neq(TextColor(colors.foreground));
        if caret.color != colors.caret {
            caret.color = colors.caret;
        }
        if caret.selection_color != colors.selection_background {
            caret.selection_color = colors.selection_background;
        }
        if caret.selected_text_color != Some(colors.selection_foreground) {
            caret.selected_text_color = Some(colors.selection_foreground);
        }
        background.set_if_neq(BackgroundColor(theme.segmented.container.normal.background));
        border.set_if_neq(BorderColor::all(if disabled {
            Color::srgb_u8(110, 120, 135)
        } else {
            Color::srgb_u8(40, 200, 180)
        }));
        for (owner, mut label) in &mut labels {
            if owner.0 == parent.parent() {
                label.set_if_neq(Text(tr_args(
                    *language,
                    TextKey::TextField(TextFieldKey::CustomContent),
                    &[("value", edit.value().to_string())],
                )));
            }
        }
    }
    Ok(())
}
