use crate::gallery::{GalleryPage, mount_page, unmount_page};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::text_field::{
    AutoSize, WidgetryReadOnlyTextField, WidgetryTextField, WidgetryTextFieldPrefix,
    WidgetryTextFieldSuffix,
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

fn toggle_auto_width(
    _event: On<bevy::ui_widgets::Activate>,
    mut fields: Query<(&Name, &mut Node)>,
) -> Result {
    let (_, mut node) = fields
        .iter_mut()
        .find(|(name, _)| name.as_str() == "TextFieldAutoSize")
        .ok_or_else(|| BevyError::error("AutoSize 演示输入框缺失"))?;
    let width = if node.width == px(480) { 240.0 } else { 480.0 };
    node.width = px(width);
    info!(width, "切换 AutoSize 输入框宽度");
    Ok(())
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
        @WidgetryTextField { @value: {("Single line").to_owned()}, @show_count: true }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryTextField { @value: {("First line\nSecond line\nThird line\nFourth line").to_owned()}, @visible_lines: {Some(4.0)}, @multiline: true }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryReadOnlyTextField { @value: {("Read-only text: select and copy me").to_owned()}, @show_count: true, @hint_text: {Some("Read-only: selection and copy remain available.".to_owned())} }
            Node { margin: UiRect::bottom(px(16)) }
        --
        @WidgetryTextField { @value: {("Disabled").to_owned()}, @hint_text: {Some("Disabled: this warning stays visible.".to_owned())} } InteractionDisabled
        --
        @WidgetryTextField { @value: {"甲e\u{301}👍🏽".to_owned()}, @max_length: {Some(5)}, @show_count: true }
            Name("TextFieldMaxLength")
            Node { margin: UiRect::top(px(16)), width: px(360) }
        --
        @WidgetryTextField {
            @value: {"editable".to_owned()}, @max_length: {Some(12)}, @show_count: true,
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
        --
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text("AutoSize: narrow / wide") bevy_widgetry::text::WidgetryText}) as Box<dyn SceneList>)} }
        Node {margin:UiRect::top(px(16)),align_self:AlignSelf::Start}
        on(toggle_auto_width)
        --
        @WidgetryTextField {
            @value: {"AutoSize grows with wrapped text. Type more words or add new lines.".to_owned()},
            @multiline: true, @auto_size: {Some(AutoSize::default())}, @show_count: true,
            @max_length: {Some(160)},
            @hint_text: {Some("Use up to 160 graphemes. This warning stays visible and wraps independently of the editor. Help: https://example.com/verylongunbrokenreferencewithoutspacesverylongunbrokenreferencewithoutspaces".to_owned())},
            @children: {Some(Box::new(bsn_list! {
                WidgetryTextFieldPrefix Children [Text("Note ") bevy_widgetry::text::WidgetryText]--
                WidgetryTextFieldSuffix Children [
                    @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text("Clear") bevy_widgetry::text::WidgetryText}) as Box<dyn SceneList>)} }
                    on(clear_slot_text)
                ]
            }) as Box<dyn SceneList>)},
        }
        Name("TextFieldAutoSize")
        Node { margin: UiRect::top(px(16)), width: px(480) }
        --
        @WidgetryReadOnlyTextField {
            @value: {"Read-only AutoSize\nSecond line\nThird line\nFourth line".to_owned()},
            @multiline: true, @auto_size: {Some(AutoSize {min_rows:1,max_rows:3})},
        }
        Node { margin: UiRect::top(px(16)), width: px(480) }
    }
}
