use super::overrides::{TEAL, icon};
use super::pair;
use crate::i18n::{LocalizedText, TextKey, localized, localized_args};
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
            @pair(TextKey::Text,TextKey::FixedColorConvenienceApi0F766ERetainedAcross, text_button(), text_button(), text_colors)--
            @pair(TextKey::Icon,TextKey::FixedColorConvenienceApi0F766ERetainedAcross, icon_button(), icon_button(), icon_colors)--
            @localized(TextKey::NestedNodeContentRootButtonDisabledAffects) WidgetryText--
            @WidgetryButton template(|_| Ok(CompositionButton))
                Name("ColorCompositionButton") on(activated)
                Children [Node Children [Node { column_gap: px(8), align_items: AlignItems::Center }
                    Children [@icon()-- @localized(TextKey::ManagedText) WidgetryText-- @localized(TextKey::NativeFixedText) TextColor(TEAL)]]]--
            @WidgetryButton Name("ColorCompositionToggle") on(toggle_button) Children [@localized(TextKey::ToggleRootButtonDisabled) WidgetryText]--
            template(|_| Ok(Activations(0))) Name("ColorCompositionCount") @localized_args(TextKey::ButtonActivations, vec![("count", "0".into())]) WidgetryText--
            @localized(TextKey::DisabledContainersAffectDescendantsThroughPlainNodes) WidgetryText--
            template(|_| Ok(DisabledContainer)) Name("ColorDisabledContainer") InteractionDisabled
                Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8) }
                Children [Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                    @WidgetryCheckBox Name("ColorInheritedCheckBox") Children [@localized(TextKey::InheritParentDisabled) WidgetryText]--
                    @WidgetryTextField Name("ColorInheritedTextField") ~{EditableText::new("父级恢复后可以编辑")}--
                    @WidgetryCheckBox Name("ColorLocalDisabledCheckBox") InteractionDisabled Children [@localized(TextKey::LocallyDisabledStaysDisabledAfterParentIs) WidgetryText]
                ]]--
            @WidgetryButton Name("ColorDisabledContainerToggle") on(toggle_container) Children [@localized(TextKey::ToggleContainerDisabled) WidgetryText]
        ]
    }
}

fn text_button() -> impl Scene {
    bsn! { @WidgetryButton Children [Node Children [Node Children [template(|_| Ok(ContentText)) @localized(TextKey::TextInsideButton) WidgetryText]]] }
}

fn icon_button() -> impl Scene {
    bsn! { @WidgetryButton Children [Node Children [Node { column_gap: px(8) } Children [@icon() template(|_| Ok(ContentIcon))-- @localized(TextKey::IconInsideButton) WidgetryText]]] }
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

fn activated(_: On<Activate>, mut counts: Query<(&mut Activations, &mut LocalizedText)>) {
    for (mut count, mut text) in &mut counts {
        count.0 += 1;
        *text = LocalizedText::with_args(
            TextKey::ButtonActivations,
            vec![("count", count.0.to_string())],
        );
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
