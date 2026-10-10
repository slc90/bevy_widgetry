use crate::gallery::{GalleryPage, mount_page, unmount_page};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::text_field::{WidgetryReadOnlyTextField, WidgetryTextField};

pub(crate) struct TextFieldDemoPlugin;

impl Plugin for TextFieldDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GalleryPage::TextField), enter)
            .add_systems(OnExit(GalleryPage::TextField), exit);
    }
}

fn enter(world: &mut World) -> Result {
    mount_page(world, GalleryPage::TextField, scene())?;
    Ok(())
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::TextField)
}

pub(crate) fn scene() -> impl SceneList {
    bsn_list! {
        @WidgetryTextField
            ~{EditableText::new("Single line")}
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryTextField
            ~{EditableText::new("First line\nSecond line\nThird line\nFourth line")}
            EditableText { visible_lines: {Some(4.0)}, allow_newlines: true }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryReadOnlyTextField
            ~{EditableText::new("Read-only text: select and copy me")}
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryTextField ~{EditableText::new("Disabled")} InteractionDisabled
    }
}
