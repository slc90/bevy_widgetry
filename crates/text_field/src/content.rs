use crate::{
    TextFieldError, TextFieldFailure, WidgetryReadOnlyTextField, WidgetryTextField,
    error::{failure, missing},
    length::LengthLimit,
    text_field::Parts,
};
use bevy::{
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_diagnostics::ErrorFrame;

fn validate<T: Component>(
    world: &World,
    root: Entity,
    operation: &'static str,
) -> Result<(Entity, Option<usize>), TextFieldFailure> {
    if world.get::<T>(root).is_none() {
        return Err(failure(
            TextFieldError::InvalidRoot {
                root,
                expected: std::any::type_name::<T>(),
            },
            ErrorFrame::new(operation),
        ));
    }
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| missing(root, root, "Parts", operation))?;
    let context = || {
        ErrorFrame::new(operation)
            .field("root", root)
            .field("editor", parts.editor)
            .field("frame", parts.frame)
    };
    if world.get::<EditableText>(parts.editor).is_none() {
        return Err(missing(root, parts.editor, "EditableText", operation).with_context(context()));
    }
    let limit = world
        .get::<LengthLimit>(root)
        .ok_or_else(|| missing(root, root, "LengthLimit", operation).with_context(context()))?
        .0;
    Ok((parts.editor, limit))
}

fn text<T: Component>(world: &World, root: Entity) -> Result<String, TextFieldFailure> {
    let (editor, _) = validate::<T>(world, root, "text_field.text")?;
    world
        .get::<EditableText>(editor)
        .map(|edit| edit.value().to_string())
        .ok_or_else(|| missing(root, editor, "EditableText", "text_field.text"))
}

fn set_text<T: Component>(
    world: &mut World,
    root: Entity,
    value: &str,
) -> Result<bool, TextFieldFailure> {
    let (editor, max) = validate::<T>(world, root, "text_field.set_text")?;
    for (present, resource) in [
        (world.contains_resource::<FontCx>(), "FontCx"),
        (world.contains_resource::<LayoutCx>(), "LayoutCx"),
    ] {
        if !present {
            return Err(failure(
                TextFieldError::MissingResource { root, resource },
                ErrorFrame::new("text_field.set_text").field("editor", editor),
            ));
        }
    }
    let value = crate::length::truncate(value, max);
    let changed = world.resource_scope(|world, mut fonts: Mut<FontCx>| {
        world.resource_scope(|world, mut layout: Mut<LayoutCx>| {
            let mut edit = world
                .get_mut::<EditableText>(editor)
                .ok_or_else(|| missing(root, editor, "EditableText", "text_field.set_text"))?;
            let changed = edit.value() != value;
            // set_text drops the composition range without restoring a hidden IME caret.
            // End the composition through the official driver before replacing its buffer.
            edit.editor_mut()
                .driver(&mut fonts, &mut layout.0)
                .clear_compose();
            let selection = *edit.editor().raw_selection();
            let anchor = selection.anchor().index();
            let focus = selection.focus().index();
            edit.pending_edits.clear();
            edit.pending_paste = None;
            edit.editor_mut().set_text(value);
            let mut driver = edit.editor_mut().driver(&mut fonts, &mut layout.0);
            if value.is_char_boundary(anchor) && value.is_char_boundary(focus) {
                driver.move_to_byte(anchor);
                driver.extend_selection_to_byte(focus);
            } else {
                driver.move_to_text_end();
            }
            Ok::<_, TextFieldFailure>(changed)
        })
    })?;
    crate::count::refresh_in_world(world, root).map_err(|error| {
        error.with_context(
            ErrorFrame::new("text_field.set_text.count_projection")
                .field("root", root)
                .field("editor", editor)
                .field("text_written", true)
                .field("content_changed", changed),
        )
    })?;
    Ok(changed)
}

macro_rules! content_api {
    ($widget:ty) => {
        impl $widget {
            pub fn editor(world: &World, root: Entity) -> Result<&EditableText, TextFieldFailure> {
                let (editor, _) = validate::<Self>(world, root, "text_field.editor")?;
                world
                    .get::<EditableText>(editor)
                    .ok_or_else(|| missing(root, editor, "EditableText", "text_field.editor"))
            }
            pub fn focus_in_world(world: &mut World, root: Entity) -> Result<(), TextFieldFailure> {
                let (editor, _) = validate::<Self>(world, root, "text_field.focus")?;
                world
                    .get_resource_mut::<bevy::input_focus::InputFocus>()
                    .ok_or_else(|| {
                        failure(
                            TextFieldError::MissingResource {
                                root,
                                resource: "InputFocus",
                            },
                            ErrorFrame::new("text_field.focus").field("editor", editor),
                        )
                    })?
                    .set(editor, bevy::input_focus::FocusCause::Navigated);
                Ok(())
            }
            pub fn focus(commands: &mut Commands, root: Entity) {
                commands.queue(move |world: &mut World| {
                    Self::focus_in_world(world, root).map_err(|error| error.into_bevy_error())
                });
            }
            pub fn text(world: &World, root: Entity) -> Result<String, TextFieldFailure> {
                text::<Self>(world, root)
            }
            pub fn set_text_in_world(
                world: &mut World,
                root: Entity,
                value: impl AsRef<str>,
            ) -> Result<bool, TextFieldFailure> {
                set_text::<Self>(world, root, value.as_ref())
            }
            pub fn set_text(commands: &mut Commands, root: Entity, value: impl Into<String>) {
                let value = value.into();
                commands.queue(move |world: &mut World| {
                    Self::set_text_in_world(world, root, &value)
                        .map(|_| ())
                        .map_err(|error| error.into_bevy_error())
                });
            }
        }
    };
}
content_api!(WidgetryTextField);
content_api!(WidgetryReadOnlyTextField);

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_core::scene::spawn_scene;

    #[test]
    fn content_mapping_distinguishes_parts_and_length_limit_for_both_variants() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        for read_only in [false, true] {
            for remove_parts in [false, true] {
                let root = if read_only {
                    spawn_scene(app.world_mut(), bsn! { @WidgetryReadOnlyTextField }).unwrap()
                } else {
                    spawn_scene(app.world_mut(), bsn! { @WidgetryTextField }).unwrap()
                };
                if remove_parts {
                    app.world_mut().entity_mut(root).remove::<Parts>();
                } else {
                    app.world_mut().entity_mut(root).remove::<LengthLimit>();
                }
                let error = if read_only {
                    WidgetryReadOnlyTextField::text(app.world(), root)
                } else {
                    WidgetryTextField::text(app.world(), root)
                }
                .unwrap_err();
                assert!(
                    matches!(error.kind(), TextFieldError::MissingPart { root: owner, entity, component } if *owner == root && *entity == root && *component == if remove_parts { "Parts" } else { "LengthLimit" })
                );
                assert!(
                    error
                        .contexts()
                        .iter()
                        .any(|frame| frame.operation() == "text_field.text")
                );
            }
        }
    }
}
