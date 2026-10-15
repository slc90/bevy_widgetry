use crate::assets::GalleryIcon;
use crate::i18n::{Language, LocalizedText, TextKey, localized, tr};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry::{
    button::*, check_box::*, icon::WidgetryIcon, radio_group::*, scroll_area::*,
    text::WidgetryText, text_field::*, tooltip::*,
};

pub(crate) const PURPLE: Color = Color::srgb_u8(124, 58, 237);
pub(crate) const TEAL: Color = Color::srgb_u8(15, 118, 110);

type Apply = fn(&mut World, Entity, bool) -> Result<bool, BevyError>;

#[derive(Component)]
struct Pair;

#[derive(Component)]
struct PairBody;

#[derive(Component, Clone, Copy)]
struct Target(Apply);

#[derive(Component, Clone, Copy, Debug)]
enum Action {
    Apply,
    Clear,
    Disabled,
}

#[derive(Component)]
struct Status;

pub(crate) fn pair(
    label: TextKey,
    description: TextKey,
    pure: impl Scene,
    sample: impl Scene,
    apply: Apply,
) -> impl Scene {
    // Erase both Widget Scene types so the large Gallery composition cannot exhaust the Windows main thread stack.
    let pure: Box<dyn Scene> = Box::new(pure);
    let sample: Box<dyn Scene> = Box::new(
        bsn! { @{sample} template(move |_| Ok(Target(apply))) Name({format!("ColorExample:{}", tr(Language::English, label))}) },
    );
    let controls: Vec<_> = [
        (TextKey::ApplySampleOverrides, Action::Apply),
        (TextKey::ClearOverrides, Action::Clear),
        (TextKey::ToggleParentDisabled, Action::Disabled),
    ]
    .into_iter()
    .map(|(text, action)| {
        bsn! {
            @WidgetryButton template(move |_| Ok(action))
            Name({format!("ColorExample:{}:{action:?}", tr(Language::English, label))})
            on(operate) Children [@localized(text) WidgetryText]
        }
    })
    .collect();
    bsn! {
        template(|_| Ok(Pair))
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), flex_shrink: 0.0 }
        Children [
            @localized(label) WidgetryText--
            @localized(description) WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }--
            template(|_| Ok(PairBody)) Node { width: percent(100), column_gap: px(20), align_items: AlignItems::Start }
                Children [
                    Node { flex_basis: px(0), flex_grow: 1.0, min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(6) } Children [@localized(TextKey::PureTheme) WidgetryText-- @{pure}]--
                    Node { flex_basis: px(0), flex_grow: 1.0, min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(6) } Children [@localized(TextKey::OverridesViaPublicApi) WidgetryText-- @{sample}]
                ]--
            Node { column_gap: px(8), flex_wrap: FlexWrap::Wrap, row_gap: px(6) } Children [{controls}]--
            template(|_| Ok(Status)) @localized(TextKey::CurrentThemeParentEnabled) WidgetryText
        ]
    }
}

fn operate(event: On<Activate>, actions: Query<&Action>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    let button = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        let mut root = button;
        while world.get::<Pair>(root).is_none() {
            let Some(parent) = world.get::<ChildOf>(root) else {
                return Ok(());
            };
            root = parent.parent();
        }
        let descendants = super::descendants(world, root);
        let mut changed = false;
        for entity in &descendants {
            match action {
                Action::Apply | Action::Clear => {
                    if let Some(target) = world.get::<Target>(*entity).copied() {
                        changed |= (target.0)(world, *entity, matches!(action, Action::Apply))?;
                    }
                }
                Action::Disabled => {
                    if world.get::<PairBody>(*entity).is_some() {
                        if world.get::<InteractionDisabled>(*entity).is_some() {
                            world.entity_mut(*entity).remove::<InteractionDisabled>();
                        } else {
                            world.entity_mut(*entity).insert(InteractionDisabled);
                        }
                        changed = true;
                    }
                }
            }
        }
        for entity in descendants {
            if world.get::<Status>(entity).is_some()
                && let Some(mut text) = world.get_mut::<LocalizedText>(entity)
            {
                *text = LocalizedText::with_args(
                    TextKey::OverrideStatus,
                    vec![
                        ("action", format!("{action:?}")),
                        ("changed", changed.to_string()),
                    ],
                );
            }
        }
        info!(?root, ?action, changed, "操作颜色覆盖示例");
        Ok(())
    });
}

pub(super) fn button_examples() -> impl Scene {
    bsn! { Node { flex_direction: FlexDirection::Column, row_gap: px(16) } Children [
        @pair(TextKey::Button,TextKey::OnlyHoveredBackground7C3AedRealHoverPressed, button(), button(), button_colors)--
        @pair(TextKey::RadioGroup,TextKey::OnlyOptionCheckedNormalDot0F766EMove, radio(), radio(), radio_colors)
    ] }
}

fn button() -> impl Scene {
    bsn! { @WidgetryButton Node { column_gap: px(8), align_items: AlignItems::Center } Children [@icon()-- @localized(TextKey::HoverPress) WidgetryText] }
}

pub(super) fn icon() -> impl Scene {
    bsn! { @WidgetryIcon { @path: {GalleryIcon::ButtonStar.path()}, @max_size: {Some(UVec2::new(20, 20))} } }
}

fn button_colors(world: &mut World, entity: Entity, apply: bool) -> Result<bool, BevyError> {
    if !apply {
        return WidgetryButtonColorOverrides::clear_in_world(world, entity);
    }
    let mut colors = WidgetryButtonColorOverrides::default();
    colors.hovered.background = Some(PURPLE);
    WidgetryButtonColorOverrides::set_in_world(world, entity, colors)
}

fn radio() -> impl Scene {
    bsn! { @WidgetryRadioGroup Node { column_gap: px(16) } Children [
        @WidgetryRadioOption Children [@localized(TextKey::Apple) WidgetryText]--
        @WidgetryRadioOption Children [@localized(TextKey::Orange) WidgetryText]
    ] }
}

fn radio_colors(world: &mut World, entity: Entity, apply: bool) -> Result<bool, BevyError> {
    if !apply {
        return WidgetryRadioGroupColorOverrides::clear_in_world(world, entity);
    }
    let mut colors = WidgetryRadioGroupColorOverrides::default();
    colors.option.checked.normal.dot = Some(TEAL);
    WidgetryRadioGroupColorOverrides::set_in_world(world, entity, colors)
}

pub(super) fn check_box_examples() -> impl Scene {
    pair(
        TextKey::CheckBox,
        TextKey::OnlyCheckedNormalMark0F766EMovePointer,
        checkbox(),
        checkbox(),
        checkbox_colors,
    )
}

fn checkbox() -> impl Scene {
    bsn! { @WidgetryCheckBox Children [@localized(TextKey::CheckMe) WidgetryText] }
}

fn checkbox_colors(world: &mut World, entity: Entity, apply: bool) -> Result<bool, BevyError> {
    if !apply {
        return WidgetryCheckBoxColorOverrides::clear_in_world(world, entity);
    }
    let mut colors = WidgetryCheckBoxColorOverrides::default();
    colors.checked.normal.mark = Some(TEAL);
    WidgetryCheckBoxColorOverrides::set_in_world(world, entity, colors)
}

pub(super) fn text_field_examples() -> impl Scene {
    bsn! { Node { flex_direction: FlexDirection::Column, row_gap: px(16) } Children [
        @pair(TextKey::TextField,TextKey::OnlyEditableFocusedBorder7C3AedClickTo, field(false), field(false), field_colors)--
        @pair(TextKey::ReadOnly,TextKey::OnlyReadOnlyFocusedBorder7C3AedSelect, field(true), field(true), field_colors)
    ] }
}

fn field(read_only: bool) -> Box<dyn Scene> {
    if read_only {
        Box::new(
            bsn! { @WidgetryReadOnlyTextField ~{EditableText::new("ReadOnly: select and copy")} Node { width: percent(100) } },
        )
    } else {
        Box::new(
            bsn! { @WidgetryTextField ~{EditableText::new("Editable: type here")} Node { width: percent(100) } },
        )
    }
}

fn field_colors(world: &mut World, entity: Entity, apply: bool) -> Result<bool, BevyError> {
    if !apply {
        return WidgetryTextFieldColorOverrides::clear_in_world(world, entity);
    }
    let mut colors = WidgetryTextFieldColorOverrides::default();
    if world.get::<WidgetryReadOnlyTextField>(entity).is_some() {
        colors.read_only.focused.border = Some(PURPLE);
    } else {
        colors.editable.focused.border = Some(PURPLE);
    }
    WidgetryTextFieldColorOverrides::set_in_world(world, entity, colors)
}

pub(super) fn scroll_examples() -> impl Scene {
    pair(
        TextKey::ScrollArea,
        TextKey::OnlyVerticalThumbHoveredBackground7C3AedHorizontal,
        scroll(),
        scroll(),
        scroll_colors,
    )
}

fn scroll() -> impl Scene {
    bsn! { @WidgetryScrollArea { @axis: ScrollAxis::Both, @children: bsn_list!{Node { width: px(700), height: px(360), flex_shrink: 0.0 } Children [@localized(TextKey::ScrollHoverVerticalThumb) WidgetryText]} } Node { width: percent(100), height: px(160) } }
}

fn scroll_colors(world: &mut World, entity: Entity, apply: bool) -> Result<bool, BevyError> {
    if !apply {
        return WidgetryScrollAreaColorOverrides::clear_in_world(world, entity);
    }
    let mut colors = WidgetryScrollAreaColorOverrides::default();
    colors.vertical.thumb.hovered.background = Some(PURPLE);
    WidgetryScrollAreaColorOverrides::set_in_world(world, entity, colors)
}

pub(super) fn tooltip_examples() -> impl Scene {
    pair(
        TextKey::Tooltip,
        TextKey::OnlyPopupNormalBackground7C3AedRealHover,
        tooltip(),
        tooltip(),
        tooltip_colors,
    )
}

fn tooltip() -> impl Scene {
    bsn! { @WidgetryButton @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{@localized(TextKey::TooltipReadableThemeForeground) WidgetryText})} } Children [@localized(TextKey::HoverForTooltip) WidgetryText] }
}

fn tooltip_colors(world: &mut World, entity: Entity, apply: bool) -> Result<bool, BevyError> {
    if !apply {
        return WidgetryTooltipColorOverrides::clear_in_world(world, entity);
    }
    let mut colors = WidgetryTooltipColorOverrides::default();
    colors.popup.normal.background = Some(PURPLE);
    WidgetryTooltipColorOverrides::set_in_world(world, entity, colors)
}
