use super::overrides::{TEAL, icon};
use super::pair;
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry::{
    button::WidgetryButton, check_box::WidgetryCheckBox, icon::WidgetryIcon, text::WidgetryText,
    text_field::WidgetryTextField,
};

#[derive(Component)]
struct ContentText;

#[derive(Component)]
struct ContentIcon;

#[derive(Component)]
struct CompositionButton;

#[derive(Component)]
struct DisabledContainer;

#[derive(Component)]
struct Activations(usize);

pub(super) fn scene() -> impl Scene {
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            pair("Text", "固定色便利 API=#0F766E；Theme 和父级 Disabled 切换后保持。清除后继承 Button 前景色。", text_button(), text_button(), text_colors),
            pair("Icon", "固定色便利 API=#0F766E；Theme 和父级 Disabled 切换后保持。清除后继承 Button 前景色。", icon_button(), icon_button(), icon_colors),
            Text("多层 Node 内容：根 Button Disabled 影响 WidgetryText / Icon；原生固定 Text 保持青色。") WidgetryText,
            (@WidgetryButton template(|_| Ok(CompositionButton))
                Name("ColorCompositionButton") on(activated)
                Children [(Node Children [(Node { column_gap: px(8), align_items: AlignItems::Center }
                    Children [icon(), Text("Managed text") WidgetryText, (Text("Native fixed text") TextColor(TEAL))])])]),
            (@WidgetryButton Name("ColorCompositionToggle") on(toggle_button) Children [Text("切换根 Button Disabled") WidgetryText]),
            (template(|_| Ok(Activations(0))) Name("ColorCompositionCount") Text("Button Activate: 0") WidgetryText),
            Text("禁用容器穿过普通 Node；恢复父级后，仅本地单独禁用的 CheckBox 继续禁用。") WidgetryText,
            (template(|_| Ok(DisabledContainer)) Name("ColorDisabledContainer") InteractionDisabled
                Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [(Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                    (@WidgetryCheckBox Name("ColorInheritedCheckBox") Children [Text("继承父级 Disabled") WidgetryText]),
                    (@WidgetryTextField Name("ColorInheritedTextField") template_value(EditableText::new("父级恢复后可以编辑"))),
                    (@WidgetryCheckBox Name("ColorLocalDisabledCheckBox") InteractionDisabled Children [Text("本地也 Disabled，父级恢复后仍禁用") WidgetryText]),
                ])]),
            (@WidgetryButton Name("ColorDisabledContainerToggle") on(toggle_container) Children [Text("切换容器 Disabled") WidgetryText]),
        ]
    }
}

fn text_button() -> impl Scene {
    bsn! { @WidgetryButton Children [(Node Children [(Node Children [(template(|_| Ok(ContentText)) Text("Text inside Button") WidgetryText)])])] }
}

fn icon_button() -> impl Scene {
    bsn! { @WidgetryButton Children [(Node Children [(Node { column_gap: px(8) } Children [(icon() template(|_| Ok(ContentIcon))), Text("Icon inside Button") WidgetryText])])] }
}

fn text_colors(world: &mut World, root: Entity, apply: bool) -> Result<bool, BevyError> {
    let target = content::<ContentText>(world, root)?;
    if apply {
        WidgetryText::set_color_in_world(world, target, TEAL)
    } else {
        WidgetryText::clear_color_in_world(world, target)
    }
}

fn icon_colors(world: &mut World, root: Entity, apply: bool) -> Result<bool, BevyError> {
    let target = content::<ContentIcon>(world, root)?;
    if apply {
        WidgetryIcon::set_color_in_world(world, target, TEAL)
    } else {
        WidgetryIcon::clear_color_in_world(world, target)
    }
}

fn content<T: Component>(world: &World, root: Entity) -> Result<Entity, BevyError> {
    super::descendants(world, root)
        .into_iter()
        .find(|entity| world.get::<T>(*entity).is_some())
        .ok_or_else(|| {
            error!(?root, "Gallery 固定颜色实例缺少内容");
            BevyError::error("Gallery 固定颜色实例缺少内容")
        })
}

fn activated(_: On<Activate>, mut counts: Query<(&mut Activations, &mut Text)>) {
    for (mut count, mut text) in &mut counts {
        count.0 += 1;
        text.0 = format!("Button Activate: {}", count.0);
        info!(count = count.0, "激活多层内容 Button");
    }
}

fn toggle_button(
    _: On<Activate>,
    roots: Query<(Entity, Has<InteractionDisabled>), With<CompositionButton>>,
    mut commands: Commands,
) {
    for (root, disabled) in &roots {
        set_disabled(&mut commands, root, !disabled);
    }
}

fn toggle_container(
    _: On<Activate>,
    roots: Query<(Entity, Has<InteractionDisabled>), With<DisabledContainer>>,
    mut commands: Commands,
) {
    for (root, disabled) in &roots {
        set_disabled(&mut commands, root, !disabled);
    }
}

fn set_disabled(commands: &mut Commands, root: Entity, disabled: bool) {
    if disabled {
        commands.entity(root).insert(InteractionDisabled);
    } else {
        commands.entity(root).remove::<InteractionDisabled>();
    }
    info!(?root, disabled, "切换组合示例 Disabled");
}
