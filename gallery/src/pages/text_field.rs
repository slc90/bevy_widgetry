use crate::gallery::{GalleryPage, mount_page, unmount_page};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::text_field::{
    WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPrefix, WidgetryTextFieldSuffix,
};

fn clear_slot_text(
    event: On<bevy::ui_widgets::Activate>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryTextField>>,
    mut commands: Commands,
) {
    let root = parents
        .iter_ancestors(event.entity)
        .find(|&entity| roots.contains(entity));
    commands.queue(move |world: &mut World| -> Result {
        let root = root.ok_or_else(|| BevyError::error("组合 TextField 根实体缺失"))?;
        let changed = WidgetryTextField::set_text_in_world(world, root, "")?;
        info!(changed, "清空组合 TextField 正文");
        Ok(())
    });
}

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
        @WidgetryTextField { @value: {("Single line").to_owned()} }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryTextField { @value: {("First line\nSecond line\nThird line\nFourth line").to_owned()}, @visible_lines: {Some(4.0)}, @allow_newlines: true }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryReadOnlyTextField { @value: {("Read-only text: select and copy me").to_owned()} }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryTextField { @value: {("Disabled").to_owned()} } InteractionDisabled
        --
        @WidgetryTextField { @value: {"甲e\u{301}👍🏽".to_owned()}, @max_length: {Some(5)} }
            Name("TextFieldMaxLength")
            Node { margin: UiRect::top(px(16)), width: px(360) }
        --
        @WidgetryTextField {
            @value: {"editable".to_owned()}, @max_length: {Some(12)},
            @children: {Some(Box::new(bsn_list! {
                WidgetryTextFieldSuffix Children [
                    @WidgetryButton { @children: {Some(Box::new(bsn_list! { Text("Clear") bevy_widgetry::text::WidgetryText }) as Box<dyn SceneList>)} }
                    Name("TextFieldSlotClear") on(clear_slot_text)
                ]--
                WidgetryTextFieldPrefix Children [ Text("Prefix ") bevy_widgetry::text::WidgetryText ]
            }) as Box<dyn SceneList>)},
        }
        Name("TextFieldSlots") bevy::input_focus::tab_navigation::TabIndex(0)
        Node { margin: UiRect::top(px(16)), width: px(480) }
    }
}
