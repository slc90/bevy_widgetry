use crate::{WidgetryReadOnlyTextField, WidgetryTextField, length::LengthLimit};
use bevy::{
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_internal_log::widgetry_error;

fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(
        ?entity,
        "TextField 内容目标不存在、类型不匹配或内部编辑器缺失"
    );
    BevyError::error("TextField 内容目标不存在、类型不匹配或内部编辑器缺失")
}

fn validate<T: Component>(world: &World, entity: Entity) -> Result<Option<usize>, BevyError> {
    if world.get::<T>(entity).is_none() || world.get::<EditableText>(entity).is_none() {
        return Err(invalid_target(entity));
    }
    world
        .get::<LengthLimit>(entity)
        .map(|limit| limit.0)
        .ok_or_else(|| invalid_target(entity))
}

fn text<T: Component>(world: &World, entity: Entity) -> Result<String, BevyError> {
    validate::<T>(world, entity)?;
    world
        .get::<EditableText>(entity)
        .map(|edit| edit.value().to_string())
        .ok_or_else(|| invalid_target(entity))
}

fn set_text<T: Component>(
    world: &mut World,
    entity: Entity,
    value: &str,
) -> Result<bool, BevyError> {
    let max = validate::<T>(world, entity)?;
    let value = crate::length::truncate(value, max);
    world.resource_scope(|world, mut fonts: Mut<FontCx>| {
        world.resource_scope(|world, mut layout: Mut<LayoutCx>| {
            let mut edit = world
                .get_mut::<EditableText>(entity)
                .ok_or_else(|| invalid_target(entity))?;
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
            Ok(changed)
        })
    })
}

macro_rules! content_api {
    ($widget:ty) => {
        impl $widget {
            pub fn text(world: &World, entity: Entity) -> Result<String, BevyError> {
                text::<Self>(world, entity)
            }

            pub fn set_text_in_world(
                world: &mut World,
                entity: Entity,
                value: impl AsRef<str>,
            ) -> Result<bool, BevyError> {
                set_text::<Self>(world, entity, value.as_ref())
            }

            pub fn set_text(commands: &mut Commands, entity: Entity, value: impl Into<String>) {
                let value = value.into();
                commands.queue(move |world: &mut World| {
                    Self::set_text_in_world(world, entity, &value).map(|_| ())
                });
            }
        }
    };
}

content_api!(WidgetryTextField);
content_api!(WidgetryReadOnlyTextField);
