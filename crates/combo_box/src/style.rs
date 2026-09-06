use bevy::{
    app::{App, Plugin, Propagate, Update},
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        lifecycle::RemovedComponents,
        query::{Added, Changed, Has, Or, With},
        system::{Commands, Query},
    },
    picking::hover::Hovered,
    ui::{
        AlignItems, BackgroundColor, BorderColor, FlexDirection, InteractionDisabled,
        JustifyContent, Node, PositionType, Pressed, Selected, UiRect, Val, px, widget::Text,
    },
    utils::default,
};
use bevy_widgetry_core::{ForegroundColor, ForegroundColorPlugin};

use crate::headless::{
    ComboBox, ComboBoxField, ComboBoxHierarchy, ComboBoxOption, ComboBoxPlugin, ComboBoxPopup,
    spawn_headless_combo_box,
};

// -----------------------------------------------------------------------------
// Layout constants
// -----------------------------------------------------------------------------

const COMBO_BOX_WIDTH: f32 = 200.0;
const FIELD_HEIGHT: f32 = 36.0;
const OPTION_HEIGHT: f32 = 32.0;
const HORIZONTAL_PADDING: f32 = 10.0;
const BORDER_WIDTH: f32 = 1.0;

// -----------------------------------------------------------------------------
// Colors
// -----------------------------------------------------------------------------

const POPUP_BACKGROUND: Color = Color::srgb(0.10, 0.10, 0.12);
const POPUP_BORDER: Color = Color::srgb(1.0, 0.35, 0.75);

// -----------------------------------------------------------------------------
// Style-owned state / markers
// -----------------------------------------------------------------------------

/// StyledComboBox 的固定文本数据。
///
/// 第一版 options 在构造后不再动态修改。
#[derive(Component, Debug)]
struct ComboBoxOptions(Vec<String>);

/// Field 中显示当前 selection 的 Text Entity。
#[derive(Component, Debug, Default)]
struct ComboBoxFieldText;

/// Field 右侧的下拉提示图标。
#[derive(Component, Debug, Default)]
struct ComboBoxDropdownIcon;

// -----------------------------------------------------------------------------
// Resolved style values
// -----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
struct ComboBoxFieldStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

const FIELD_STYLE_DEFAULT: ComboBoxFieldStyle = ComboBoxFieldStyle {
    background: Color::srgb(0.12, 0.12, 0.14),
    border: Color::srgb(0.10, 0.80, 1.00),
    foreground: Color::WHITE,
};

const FIELD_STYLE_HOVERED: ComboBoxFieldStyle = ComboBoxFieldStyle {
    background: Color::srgb(0.16, 0.22, 0.26),
    border: Color::srgb(0.15, 0.95, 1.00),
    foreground: Color::WHITE,
};

const FIELD_STYLE_PRESSED: ComboBoxFieldStyle = ComboBoxFieldStyle {
    background: Color::srgb(0.08, 0.35, 0.45),
    border: Color::srgb(1.00, 0.75, 0.15),
    foreground: Color::WHITE,
};

const FIELD_STYLE_OPEN: ComboBoxFieldStyle = ComboBoxFieldStyle {
    background: Color::srgb(0.12, 0.28, 0.34),
    border: Color::srgb(0.20, 1.00, 0.55),
    foreground: Color::WHITE,
};

const FIELD_STYLE_DISABLED: ComboBoxFieldStyle = ComboBoxFieldStyle {
    background: Color::srgb(0.08, 0.08, 0.09),
    border: Color::srgb(0.35, 0.35, 0.38),
    foreground: Color::srgb(0.45, 0.45, 0.48),
};

#[derive(Debug, Clone, Copy, PartialEq)]
struct ComboBoxOptionStyle {
    background: Color,
    foreground: Color,
}

const OPTION_STYLE_DEFAULT: ComboBoxOptionStyle = ComboBoxOptionStyle {
    background: POPUP_BACKGROUND,
    foreground: Color::WHITE,
};

const OPTION_STYLE_SELECTED: ComboBoxOptionStyle = ComboBoxOptionStyle {
    background: Color::srgb(0.10, 0.32, 0.42),
    foreground: Color::WHITE,
};

const OPTION_STYLE_HOVERED: ComboBoxOptionStyle = ComboBoxOptionStyle {
    background: Color::srgb(0.18, 0.45, 0.65),
    foreground: Color::WHITE,
};

const OPTION_STYLE_DISABLED: ComboBoxOptionStyle = ComboBoxOptionStyle {
    background: Color::srgb(0.08, 0.08, 0.09),
    foreground: Color::srgb(0.45, 0.45, 0.48),
};

// -----------------------------------------------------------------------------
// Pure resolvers
// -----------------------------------------------------------------------------

fn resolve_combo_box_field_style(
    disabled: bool,
    open: bool,
    pressed: bool,
    hovered: bool,
) -> ComboBoxFieldStyle {
    if disabled {
        FIELD_STYLE_DISABLED
    } else if open {
        FIELD_STYLE_OPEN
    } else if pressed {
        FIELD_STYLE_PRESSED
    } else if hovered {
        FIELD_STYLE_HOVERED
    } else {
        FIELD_STYLE_DEFAULT
    }
}

fn resolve_combo_box_option_style(
    disabled: bool,
    selected: bool,
    hovered: bool,
) -> ComboBoxOptionStyle {
    if disabled {
        OPTION_STYLE_DISABLED
    } else if hovered {
        OPTION_STYLE_HOVERED
    } else if selected {
        OPTION_STYLE_SELECTED
    } else {
        OPTION_STYLE_DEFAULT
    }
}

// -----------------------------------------------------------------------------
// Layout
// -----------------------------------------------------------------------------

fn combo_box_root_node() -> Node {
    Node {
        width: px(COMBO_BOX_WIDTH),
        ..default()
    }
}

fn combo_box_field_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: px(FIELD_HEIGHT),

        flex_direction: FlexDirection::Row,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,

        padding: UiRect::axes(px(HORIZONTAL_PADDING), px(0)),
        border: UiRect::all(px(BORDER_WIDTH)),

        ..default()
    }
}

fn combo_box_popup_node() -> Node {
    Node {
        position_type: PositionType::Absolute,

        left: px(0),
        top: Val::Percent(100.0),

        width: Val::Percent(100.0),

        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Stretch,

        border: UiRect::all(px(BORDER_WIDTH)),

        ..default()
    }
}

fn combo_box_option_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: px(OPTION_HEIGHT),

        align_items: AlignItems::Center,

        padding: UiRect::axes(px(HORIZONTAL_PADDING), px(0)),

        ..default()
    }
}

// -----------------------------------------------------------------------------
// Public constructor
// -----------------------------------------------------------------------------

pub fn spawn_styled_combo_box(commands: &mut Commands, options: Vec<String>) -> Entity {
    assert!(
        !options.is_empty(),
        "StyledComboBox requires at least one option"
    );

    let combo_box = spawn_headless_combo_box(commands, options.len(), 0);

    commands.entity(combo_box).insert(ComboBoxOptions(options));

    combo_box
}

// -----------------------------------------------------------------------------
// Initial visual structure
// -----------------------------------------------------------------------------

fn setup_styled_combo_box(
    roots: Query<(Entity, &ComboBoxOptions, Has<InteractionDisabled>), Added<ComboBoxOptions>>,
    hierarchy: ComboBoxHierarchy,
    popups: Query<&Children, With<ComboBoxPopup>>,
    options: Query<(&ComboBoxOption, Has<Selected>)>,
    mut commands: Commands,
) {
    for (root, option_labels, disabled) in &roots {
        commands.entity(root).insert(combo_box_root_node());

        if let Some(field) = hierarchy.field(root) {
            let style = resolve_combo_box_field_style(disabled, false, false, false);

            commands
                .entity(field)
                .insert((
                    Hovered::default(),
                    combo_box_field_node(),
                    BackgroundColor(style.background),
                    BorderColor::all(style.border),
                    Propagate(ForegroundColor(style.foreground)),
                ))
                .with_children(|field| {
                    field.spawn((ComboBoxFieldText, Text::new(option_labels.0[0].as_str())));

                    field.spawn((ComboBoxDropdownIcon, Text::new("v")));
                });
        }

        if let Some(popup) = hierarchy.popup(root) {
            // -----------------------------------------------------------------
            // Popup
            // -----------------------------------------------------------------

            let Ok(popup_children) = popups.get(popup) else {
                continue;
            };

            commands.entity(popup).insert((
                combo_box_popup_node(),
                BackgroundColor(POPUP_BACKGROUND),
                BorderColor::all(POPUP_BORDER),
            ));

            // -----------------------------------------------------------------
            // Options
            // -----------------------------------------------------------------

            for &option_entity in popup_children.iter() {
                let Ok((option, selected)) = options.get(option_entity) else {
                    continue;
                };

                let style = resolve_combo_box_option_style(disabled, selected, false);

                commands
                    .entity(option_entity)
                    .insert((
                        Hovered::default(),
                        combo_box_option_node(),
                        BackgroundColor(style.background),
                        Propagate(ForegroundColor(style.foreground)),
                    ))
                    .with_child(Text::new(option_labels.0[option.index].as_str()));
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn combo_box_is_open(
    root: Entity,
    hierarchy: &ComboBoxHierarchy,
    popups: &Query<&Visibility, With<ComboBoxPopup>>,
) -> bool {
    hierarchy
        .popup(root)
        .and_then(|popup| popups.get(popup).ok())
        .is_some_and(|visibility| *visibility == Visibility::Visible)
}

#[derive(bevy::ecs::system::SystemParam)]
struct FieldStyles<'w, 's> {
    hierarchy: ComboBoxHierarchy<'w, 's>,
    roots: Query<'w, 's, Has<InteractionDisabled>, With<ComboBox>>,
    popups: Query<'w, 's, &'static Visibility, With<ComboBoxPopup>>,
    fields: Query<
        'w,
        's,
        (
            &'static ChildOf,
            &'static Hovered,
            Has<Pressed>,
            &'static mut BackgroundColor,
            &'static mut BorderColor,
            &'static mut Propagate<ForegroundColor>,
        ),
        With<ComboBoxField>,
    >,
}

impl FieldStyles<'_, '_> {
    /// Resolve the complete style from current ECS state.
    fn refresh(&mut self, field: Entity) {
        let Ok((parent, hovered, pressed, mut background, mut border, mut foreground)) =
            self.fields.get_mut(field)
        else {
            return;
        };
        let root = parent.parent();
        let Ok(disabled) = self.roots.get(root) else {
            return;
        };
        let open = combo_box_is_open(root, &self.hierarchy, &self.popups);
        let style = resolve_combo_box_field_style(disabled, open, pressed, hovered.0);
        background.0 = style.background;
        *border = BorderColor::all(style.border);
        foreground.0 = ForegroundColor(style.foreground);
    }
    fn refresh_root(&mut self, root: Entity) {
        if let Some(field) = self.hierarchy.field(root) {
            self.refresh(field);
        }
    }
}

fn update_combo_box_field_style_changed(
    changed: Query<Entity, (With<ComboBoxField>, Or<(Changed<Hovered>, Added<Pressed>)>)>,
    mut styles: FieldStyles,
) {
    for field in &changed {
        styles.refresh(field);
    }
}

fn update_combo_box_field_style_pressed_removed(
    mut removed: RemovedComponents<Pressed>,
    mut styles: FieldStyles,
) {
    for field in removed.read() {
        styles.refresh(field);
    }
}

fn update_combo_box_field_style_popup_changed(
    changed: Query<&ChildOf, (With<ComboBoxPopup>, Changed<Visibility>)>,
    mut styles: FieldStyles,
) {
    for parent in &changed {
        styles.refresh_root(parent.parent());
    }
}

fn update_combo_box_field_style_disabled_changed(
    added: Query<Entity, (With<ComboBox>, Added<InteractionDisabled>)>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mut styles: FieldStyles,
) {
    for root in &added {
        styles.refresh_root(root);
    }
    for root in removed.read() {
        styles.refresh_root(root);
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct OptionStyles<'w, 's> {
    hierarchy: ComboBoxHierarchy<'w, 's>,
    roots: Query<'w, 's, Has<InteractionDisabled>, With<ComboBox>>,
    children: Query<'w, 's, &'static Children>,
    options: Query<
        'w,
        's,
        (
            &'static Hovered,
            Has<Selected>,
            &'static mut BackgroundColor,
            &'static mut Propagate<ForegroundColor>,
        ),
        With<ComboBoxOption>,
    >,
}

impl OptionStyles<'_, '_> {
    fn refresh(&mut self, option: Entity) {
        let Some(root) = self.hierarchy.root_from_option(option) else {
            return;
        };
        let Ok(disabled) = self.roots.get(root) else {
            return;
        };
        let Ok((hovered, selected, mut background, mut foreground)) = self.options.get_mut(option)
        else {
            return;
        };
        let style = resolve_combo_box_option_style(disabled, selected, hovered.0);
        background.0 = style.background;
        foreground.0 = ForegroundColor(style.foreground);
    }
    fn refresh_root(&mut self, root: Entity) {
        let Some(popup) = self.hierarchy.popup(root) else {
            return;
        };
        let Ok(children) = self.children.get(popup) else {
            return;
        };
        let options: Vec<Entity> = children.iter().copied().collect();
        for option in options {
            self.refresh(option);
        }
    }
}

fn update_combo_box_option_style_changed(
    changed: Query<
        Entity,
        (
            With<ComboBoxOption>,
            Or<(Changed<Hovered>, Changed<Selected>)>,
        ),
    >,
    mut styles: OptionStyles,
) {
    for option in &changed {
        styles.refresh(option);
    }
}

fn update_combo_box_option_style_selected_removed(
    mut removed: RemovedComponents<Selected>,
    mut styles: OptionStyles,
) {
    for option in removed.read() {
        styles.refresh(option);
    }
}

fn update_combo_box_option_style_disabled_changed(
    added: Query<Entity, (With<ComboBox>, Added<InteractionDisabled>)>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mut styles: OptionStyles,
) {
    for root in &added {
        styles.refresh_root(root);
    }
    for root in removed.read() {
        styles.refresh_root(root);
    }
}

// -----------------------------------------------------------------------------
// Field text synchronization
// -----------------------------------------------------------------------------

/// Field Text 从真正的 Selected 状态派生，而不是从 ValueChange 事件派生。
fn update_combo_box_field_text(
    selected_options: Query<(Entity, &ComboBoxOption), Changed<Selected>>,
    hierarchy: ComboBoxHierarchy,
    roots: Query<&ComboBoxOptions, With<ComboBox>>,
    fields: Query<&Children, With<ComboBoxField>>,
    mut field_texts: Query<&mut Text, With<ComboBoxFieldText>>,
) {
    for (entity, option) in &selected_options {
        let Some(root) = hierarchy.root_from_option(entity) else {
            continue;
        };
        let Ok(option_labels) = roots.get(root) else {
            continue;
        };

        let Some(label) = option_labels.0.get(option.index) else {
            continue;
        };

        if let Some(child) = hierarchy.field(root) {
            let Ok(field_children) = fields.get(child) else {
                continue;
            };

            for &field_child in field_children.iter() {
                let Ok(mut text) = field_texts.get_mut(field_child) else {
                    continue;
                };

                text.0.clone_from(label);
                break;
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Plugin
// -----------------------------------------------------------------------------

pub struct StyledComboBoxPlugin;

impl Plugin for StyledComboBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ComboBoxPlugin>() {
            app.add_plugins(ComboBoxPlugin);
        }

        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }

        app.add_systems(
            Update,
            (
                setup_styled_combo_box,
                update_combo_box_field_style_changed,
                update_combo_box_field_style_pressed_removed,
                update_combo_box_field_style_popup_changed,
                update_combo_box_field_style_disabled_changed,
                update_combo_box_option_style_changed,
                update_combo_box_option_style_selected_removed,
                update_combo_box_option_style_disabled_changed,
                update_combo_box_field_text,
            ),
        );
    }
}

// -----------------------------------------------------------------------------
// Resolver tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_field_default() {
        assert_eq!(
            resolve_combo_box_field_style(false, false, false, false),
            FIELD_STYLE_DEFAULT
        );
    }

    #[test]
    fn resolves_field_hovered() {
        assert_eq!(
            resolve_combo_box_field_style(false, false, false, true),
            FIELD_STYLE_HOVERED
        );
    }

    #[test]
    fn resolves_field_pressed_over_hovered() {
        assert_eq!(
            resolve_combo_box_field_style(false, false, true, true),
            FIELD_STYLE_PRESSED
        );
    }

    #[test]
    fn resolves_field_open_over_pressed_and_hovered() {
        assert_eq!(
            resolve_combo_box_field_style(false, true, true, true),
            FIELD_STYLE_OPEN
        );
    }

    #[test]
    fn resolves_field_disabled_over_everything() {
        assert_eq!(
            resolve_combo_box_field_style(true, true, true, true),
            FIELD_STYLE_DISABLED
        );
    }

    #[test]
    fn resolves_option_default() {
        assert_eq!(
            resolve_combo_box_option_style(false, false, false),
            OPTION_STYLE_DEFAULT
        );
    }

    #[test]
    fn resolves_option_selected() {
        assert_eq!(
            resolve_combo_box_option_style(false, true, false),
            OPTION_STYLE_SELECTED
        );
    }

    #[test]
    fn resolves_option_hovered_over_selected() {
        assert_eq!(
            resolve_combo_box_option_style(false, true, true),
            OPTION_STYLE_HOVERED
        );
    }

    #[test]
    fn resolves_option_disabled_over_everything() {
        assert_eq!(
            resolve_combo_box_option_style(true, true, true),
            OPTION_STYLE_DISABLED
        );
    }
}
