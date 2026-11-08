const EXAMPLE_BOTTOM_MARGIN: f32 = 20.0;
const EXPANDED_EDITOR_WIDTH: f32 = 480.0;
const EDITOR_PADDING: f32 = 10.0;
const EDITOR_CORNER_RADIUS: f32 = 12.0;
const ACCENT_WIDTH: f32 = 24.0;
const ACCENT_HEIGHT: f32 = 150.0;
const COMPACT_EDITOR_WIDTH: f32 = 240.0;

const DETAIL_GAP: f32 = 8.0;
const ITEM_GAP: f32 = 10.0;
const CONTROL_GAP: f32 = 8.0;
const CONTENT_GAP: f32 = 12.0;
const LABEL_FONT_SIZE: f32 = 14.0;
const TITLE_FONT_SIZE: f32 = 18.0;

use crate::i18n::{TextFieldKey, TextKey, localized};
use bevy::prelude::*;
use bevy_widgetry::{
    button::WidgetryButton,
    text::WidgetryText,
    text_field::{
        AutoSize, WidgetryTextFieldAutoSize, WidgetryTextFieldEditor, WidgetryTextFieldHeadless,
    },
    theme::WidgetryTheme,
};

#[derive(Component)]
struct Notes;

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Width,
    Fill,
    Clear,
}

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node {width:percent(100),flex_direction:FlexDirection::Column,row_gap:px(ITEM_GAP),margin:UiRect::bottom(px(EXAMPLE_BOTTOM_MARGIN))}
        Children [
            @localized(TextKey::TextField(TextFieldKey::AutoSizeDescription)) WidgetryText--
            #Root @WidgetryTextFieldHeadless {editor:#Editor}
            @WidgetryTextFieldAutoSize { @rows:{AutoSize {min_rows:2,max_rows:5}} }
            Name("TextFieldCustomAutoSize") template(|_|Ok(Notes))
            Node {width:px(EXPANDED_EDITOR_WIDTH),max_width:percent(100),column_gap:px(CONTENT_GAP),align_items:AlignItems::Center,padding:UiRect::all(px(EDITOR_PADDING)),border_radius:BorderRadius::all(px(EDITOR_CORNER_RADIUS))}
            BackgroundColor
            Children [
                Name("TextFieldCustomAutoDecoration") Pickable::IGNORE
                Node {width:px(ACCENT_WIDTH),height:px(ACCENT_HEIGHT),flex_shrink:0.0,border_radius:BorderRadius::all(px(EDITOR_CORNER_RADIUS))}
                BackgroundColor({Color::srgb_u8(40,200,180)})--
                Node {flex_grow:1.0,min_width:px(0),align_items:AlignItems::Center}
                Children [
                    #Editor @WidgetryTextFieldEditor {root:#Root,@multiline:true,@value:{"A short note. Make the panel narrow to see soft wrapping.".to_owned()}}
                    Name("TextFieldCustomAutoEditor") TextFont {font_size:FontSize::Px(TITLE_FONT_SIZE)}
                    Node {width:percent(100),min_width:px(0),align_self:AlignSelf::Center}
                ]
            ]--
            @localized(TextKey::TextField(TextFieldKey::AutoSizeHint)) WidgetryText TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}--
            Node {column_gap:px(CONTROL_GAP),row_gap:px(DETAIL_GAP),flex_wrap:FlexWrap::Wrap}
            Children [
                @control(Action::Width,TextFieldKey::AutoSizeWidth,"TextFieldCustomAutoWidth")--
                @control(Action::Fill,TextFieldKey::AutoSizeFill,"TextFieldCustomAutoFill")--
                @control(Action::Clear,TextFieldKey::CustomClear,"TextFieldCustomAutoClear")
            ]
        ]
    }
}

fn control(action: Action, key: TextFieldKey, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryButton {@children:{Some(Box::new(bsn_list!{@localized(TextKey::TextField(key)) WidgetryText TextFont {font_size:FontSize::Px(LABEL_FONT_SIZE)}}) as Box<dyn SceneList>)}}
        template(move |_|Ok(action)) Name(name) on(operate)
    }
}

fn operate(
    event: On<bevy::ui_widgets::Activate>,
    actions: Query<&Action>,
    roots: Query<Entity, With<Notes>>,
    mut commands: Commands,
) -> Result {
    let Ok(action) = actions.get(event.entity).copied() else {
        return Ok(());
    };
    let root = roots.single().map_err(crate::diagnostics::error)?;
    commands.queue(move |world:&mut World|->Result {
        let changed=match action {
            Action::Width=>{
                let mut node=world.get_mut::<Node>(root).ok_or_else(||crate::diagnostics::error("自适应笔记根布局缺失"))?;
                node.width=if node.width==px(EXPANDED_EDITOR_WIDTH) {px(COMPACT_EDITOR_WIDTH)} else {px(EXPANDED_EDITOR_WIDTH)};
                None
            }
            Action::Fill=>Some(WidgetryTextFieldHeadless::set_text_in_world(world,root,"Notes grow with text and soft wrapping. Change the width, select text, or scroll to read the rest.\nSecond paragraph.\nThird paragraph.\nFourth paragraph.\nFifth paragraph.\nSixth paragraph.\nLast line.").map_err(|error|error.into_bevy_error())?),
            Action::Clear=>Some(WidgetryTextFieldHeadless::set_text_in_world(world,root,"").map_err(|error|error.into_bevy_error())?),
        };
        info!(?root,?action,?changed,"操作自适应笔记");
        Ok(())
    });
    Ok(())
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        colors
            .before(bevy::text::detect_text_needs_rerender)
            .before(bevy::ui::UiSystems::Content)
            .run_if(in_state(crate::gallery::GalleryPage::TextField)),
    );
}

fn colors(
    theme: Res<WidgetryTheme>,
    mut roots: Query<(&WidgetryTextFieldHeadless, &mut BackgroundColor), With<Notes>>,
    mut editors: Query<(&mut TextColor, &mut bevy::text::TextCursorStyle)>,
) -> Result {
    for (binding, mut background) in &mut roots {
        background.set_if_neq(BackgroundColor(theme.segmented.container.normal.background));
        let (mut text, mut caret) = editors
            .get_mut(binding.editor)
            .map_err(crate::diagnostics::error)?;
        let colors = theme.text_field.editable.normal;
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
    }
    Ok(())
}
