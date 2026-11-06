use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    picking::hover::Hovered,
    prelude::*,
    text::{EditableText, TextCursorStyle},
};
use bevy_widgetry_core::input_target::WidgetryInputTarget;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTextFieldHeadlessProps)]
#[component(immutable, on_insert = bind_input_target)]
#[require(Node, Hovered, WidgetryInputTarget = WidgetryInputTarget {target:Entity::PLACEHOLDER})]
pub struct WidgetryTextFieldHeadless {
    pub editor: Entity,
}

#[derive(Default)]
pub struct WidgetryTextFieldHeadlessProps {
    pub read_only: bool,
}

impl WidgetryTextFieldHeadless {
    fn scene(props: WidgetryTextFieldHeadlessProps) -> impl Scene {
        let read_only = props
            .read_only
            .then_some(bsn! { crate::text_field::ReadOnly });
        bsn! { @{read_only} }
    }
}

fn bind_input_target(mut world: DeferredWorld, context: HookContext) {
    if let Some(editor) = world
        .get::<WidgetryTextFieldHeadless>(context.entity)
        .map(|binding| binding.editor)
        && let Some(mut target) = world.get_mut::<WidgetryInputTarget>(context.entity)
    {
        target.target = editor;
    }
}

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTextFieldEditorProps)]
#[component(immutable)]
pub struct WidgetryTextFieldEditor {
    pub root: Entity,
}

pub struct WidgetryTextFieldEditorProps {
    pub value: String,
    pub max_length: Option<usize>,
    pub multiline: bool,
    pub visible_lines: Option<f32>,
    pub visible_width: Option<f32>,
}
impl Default for WidgetryTextFieldEditorProps {
    fn default() -> Self {
        Self {
            value: String::new(),
            max_length: None,
            multiline: false,
            visible_lines: Some(1.0),
            visible_width: None,
        }
    }
}
impl WidgetryTextFieldEditor {
    fn scene(props: WidgetryTextFieldEditorProps) -> impl Scene {
        let value = crate::length::truncate(&props.value, props.max_length).to_owned();
        let mut editable = if value.is_empty() {
            EditableText::default()
        } else {
            EditableText::new(value)
        };
        editable.allow_newlines = props.multiline;
        editable.visible_lines = props.visible_lines;
        editable.visible_width = props.visible_width;
        bsn! {
            ~{editable} bevy::ui_widgets::TextInput TextCursorStyle
            template(move |_|Ok(crate::length::LengthLimit(props.max_length)))
            template(|_|Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Text)))
        }
    }
}

impl WidgetryTextFieldHeadless {
    pub fn editor_entity(world: &World, root: Entity) -> Result<Entity, crate::TextFieldFailure> {
        let binding = world.get::<Self>(root).ok_or_else(|| {
            crate::error::failure(
                crate::TextFieldError::InvalidRoot {
                    root,
                    expected: std::any::type_name::<Self>(),
                },
                bevy_widgetry_diagnostics::ErrorFrame::new("text_field.editor_binding"),
            )
        })?;
        let editor = binding.editor;
        let owner = world
            .get::<WidgetryTextFieldEditor>(editor)
            .ok_or_else(|| {
                crate::error::missing(
                    root,
                    editor,
                    "WidgetryTextFieldEditor",
                    "text_field.editor_binding",
                )
            })?;
        if owner.root != root {
            return Err(crate::error::failure(
                crate::TextFieldError::InvalidEditorOwner {
                    root,
                    editor,
                    actual: owner.root,
                },
                bevy_widgetry_diagnostics::ErrorFrame::new("text_field.editor_binding"),
            ));
        }
        let mut ancestor = editor;
        while ancestor != root {
            let Some(parent) = world.get::<ChildOf>(ancestor) else {
                return Err(crate::error::failure(
                    crate::TextFieldError::EditorOutsideRoot { root, editor },
                    bevy_widgetry_diagnostics::ErrorFrame::new("text_field.editor_binding"),
                ));
            };
            ancestor = parent.parent();
        }
        if world.get::<EditableText>(editor).is_none() {
            return Err(crate::error::missing(
                root,
                editor,
                "EditableText",
                "text_field.editor_binding",
            ));
        }
        if world.get::<bevy::ui_widgets::TextInput>(editor).is_none() {
            return Err(crate::error::missing(
                root,
                editor,
                "TextInput",
                "text_field.editor_binding",
            ));
        }
        Ok(editor)
    }
    pub fn is_read_only(world: &World, root: Entity) -> Result<bool, crate::TextFieldFailure> {
        Self::editor_entity(world, root)?;
        Ok(world.get::<crate::text_field::ReadOnly>(root).is_some())
    }
}
impl WidgetryTextFieldEditor {
    pub fn owner(world: &World, editor: Entity) -> Result<Entity, crate::TextFieldFailure> {
        let root = world
            .get::<Self>(editor)
            .ok_or_else(|| {
                crate::error::missing(
                    editor,
                    editor,
                    "WidgetryTextFieldEditor",
                    "text_field.editor_owner",
                )
            })?
            .root;
        let bound = WidgetryTextFieldHeadless::editor_entity(world, root)?;
        if bound != editor {
            return Err(crate::error::failure(
                crate::TextFieldError::InvalidEditorBinding {
                    root,
                    editor,
                    bound,
                },
                bevy_widgetry_diagnostics::ErrorFrame::new("text_field.editor_owner"),
            ));
        }
        Ok(root)
    }
}
