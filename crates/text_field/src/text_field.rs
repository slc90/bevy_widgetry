use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::EditableText;

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryTextFieldProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryTextField;

#[derive(SceneComponent, Default, Clone)]
#[scene(crate::WidgetryTextFieldProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryReadOnlyTextField;

#[derive(Component, Default, Clone)]
pub(super) struct TextFieldBase;

#[derive(Component, Default, Clone)]
pub(super) struct ReadOnly;

#[derive(Component, Default, Clone)]
#[require(Node = slot_node())]
pub struct WidgetryTextFieldPrefix;

#[derive(Component, Default, Clone)]
#[require(Node = slot_node())]
pub struct WidgetryTextFieldSuffix;

fn slot_node() -> Node {
    Node {
        flex_shrink: 0.0,
        align_items: AlignItems::Center,
        ..default()
    }
}

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) editor: Entity,
    pub(crate) frame: Entity,
}

pub(crate) use crate::headless::WidgetryTextFieldEditor as TextFieldEditor;

#[derive(Component, FromTemplate)]
pub(crate) struct TextFieldFrame {
    pub(crate) root: Entity,
}

impl WidgetryTextField {
    fn scene(props: crate::WidgetryTextFieldProps) -> impl Scene {
        text_field_base_scene(props, false)
    }
}

impl WidgetryReadOnlyTextField {
    fn scene(props: crate::WidgetryTextFieldProps) -> impl Scene {
        text_field_base_scene(props, true)
    }
}

// Register the BSN #TextFieldRoot reference without overwriting the caller's Name.
fn root_reference(reference: bevy::ecs::template::EntityTemplate) -> impl Scene {
    bevy::scene::SceneFunction(move |_, scene| {
        if let bevy::ecs::template::EntityTemplate::SceneEntityReference(reference) = reference {
            scene.entity_references.push(reference);
        }
    })
}

fn text_field_base_scene(props: crate::WidgetryTextFieldProps, read_only: bool) -> impl Scene {
    let auto_size = props
        .auto_size
        .map(|rows| crate::auto_size::configuration_scene(props.multiline, rows));
    let value = crate::length::truncate(&props.value, props.max_length).to_owned();
    let mut editable = if value.is_empty() {
        EditableText::default()
    } else {
        EditableText::new(value)
    };
    editable.allow_newlines = props.multiline;
    editable.visible_lines = props
        .auto_size
        .map(|config| config.min_rows as f32)
        .or(props.visible_lines);
    editable.visible_width = props.visible_width;
    let count = props.show_count.then(|| {
        let label = crate::count::label(&editable, props.max_length);
        bsn! {
        crate::count::CountDisplay
        Text(label) bevy_widgetry_core::text::WidgetryText
        TextFont { font_size: FontSize::Px(14.0) }
        TextLayout { justify: Justify::Right }
        Pickable::IGNORE
        Node { width: percent(100), flex_shrink: 0.0, margin: UiRect::top(px(4)) }
        }
    });
    let hint = props.hint_text.map(|text| {
        bsn! {
            crate::hint::HintDisplay Text(text)
            TextFont {font_size:FontSize::Px(14.0)}
        TextLayout {linebreak:bevy::text::LineBreak::WordOrCharacter}
            Pickable::IGNORE
            Node {width:percent(100),min_width:px(0),flex_shrink:0.0,margin:UiRect::top(px(4))}
        }
    });
    let editor_alignment = if props.auto_size.is_some() {
        AlignSelf::Center
    } else {
        AlignSelf::Stretch
    };
    bsn! {
        @root_reference(#TextFieldRoot)
        @{auto_size}
        TextFieldBase
        @crate::WidgetryTextFieldHeadless {editor:#TextFieldEditor,@read_only:read_only}
        Parts { editor: #TextFieldEditor, frame: #TextFieldFrame }
        template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
        Hovered(false)
        Node { flex_direction: FlexDirection::Column, min_width: px(0) }
        Children [
            #TextFieldFrame TextFieldFrame { root: #TextFieldRoot }
            Node {
                width: percent(100), min_width: px(0), min_height: px(0), flex_grow: 1.0, align_items: AlignItems::Center,
                padding: UiRect::axes(px(10), px(6)),
                border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor BorderColor
            Children [
                #TextFieldEditor @TextFieldEditor { root:#TextFieldRoot, @value:{props.value}, @max_length:{props.max_length}, @multiline:{props.multiline}, @visible_lines:{editable.visible_lines}, @visible_width:{props.visible_width} }
                Node { min_width: px(0), align_self: editor_alignment, flex_grow: 1.0, flex_basis: px(0) }--
                {props.children}
            ]
            -- {count}
            -- {hint}
        ]
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::WidgetryTextFieldPlugin;
    use bevy::scene::WorldSceneExt;
    use bevy::text::TextEdit;
    use bevy_widgetry_test_utils::scene_app;

    #[test]
    fn read_only_marker_is_exclusive_to_read_only_scene() {
        let mut app = scene_app();
        app.add_plugins(WidgetryTextFieldPlugin);
        let normal = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryTextField })
            .unwrap()
            .id();
        let read_only = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryReadOnlyTextField })
            .unwrap()
            .id();
        assert!(app.world().get::<TextFieldBase>(normal).is_some());
        assert!(app.world().get::<ReadOnly>(normal).is_none());
        assert!(app.world().get::<TextFieldBase>(read_only).is_some());
        assert!(app.world().get::<ReadOnly>(read_only).is_some());
        let normal = app.world().get::<Parts>(normal).unwrap().editor;
        app.world_mut()
            .get_mut::<EditableText>(normal)
            .unwrap()
            .queue_edit(TextEdit::Insert("X".into()));
        app.update();
        assert!(
            app.world()
                .get::<EditableText>(normal)
                .unwrap()
                .pending_edits
                .contains(&TextEdit::Insert("X".into()))
        );
    }
}
