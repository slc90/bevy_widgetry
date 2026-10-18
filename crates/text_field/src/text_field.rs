use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};

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

impl WidgetryTextField {
    fn scene(props: crate::WidgetryTextFieldProps) -> impl Scene {
        bsn! { @text_field_base_scene(props.value, props.max_length) template(move |_| props.colors.clone().initial()) }
    }
}

impl WidgetryReadOnlyTextField {
    fn scene(props: crate::WidgetryTextFieldProps) -> impl Scene {
        bsn! { @text_field_base_scene(props.value, props.max_length) ReadOnly template(move |_| props.colors.clone().initial()) }
    }
}

fn text_field_base_scene(value: String, max_length: Option<usize>) -> impl Scene {
    let value = crate::length::truncate(&value, max_length).to_owned();
    let editable = if value.is_empty() {
        EditableText::default()
    } else {
        EditableText::new(value)
    };
    bsn! {
        TextFieldBase
        template(move |_| Ok(crate::length::LengthLimit(max_length)))
        template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Text)))
        ~{editable}
        bevy::ui_widgets::TextInput
        Hovered(false)
        Node {
            padding: UiRect::axes(px(10), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor
        BorderColor
        TextCursorStyle
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
