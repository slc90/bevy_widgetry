use bevy::{
    app::{App, Plugin, Propagate, Update},
    camera::visibility::Visibility,
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        hierarchy::{ChildOf, Children},
        lifecycle::RemovedComponents,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::{Commands, ParamSet, Query, Res},
    },
    picking::hover::Hovered,
    ui::{
        AlignItems, BackgroundColor, BorderColor, FlexDirection, InteractionDisabled,
        JustifyContent, Node, PositionType, Pressed, Selected, UiRect, Val, px, widget::Text,
    },
    utils::default,
};

use bevy_widgetry_core::{
    ColorTheme, ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin,
};

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct ComboBoxOptionStyle {
    background: Color,
    foreground: Color,
}

// -----------------------------------------------------------------------------
// Pure resolvers
// -----------------------------------------------------------------------------

fn resolve_combo_box_field_style(
    colors: &ColorTheme,
    disabled: bool,
    open: bool,
    pressed: bool,
    hovered: bool,
) -> ComboBoxFieldStyle {
    let (background, border) = if disabled {
        (
            colors.control_background_disabled,
            colors.control_border_disabled,
        )
    } else if open {
        (
            colors.control_background_active,
            colors.control_border_active,
        )
    } else if pressed {
        (
            colors.control_background_pressed,
            colors.control_border_pressed,
        )
    } else if hovered {
        (
            colors.control_background_hovered,
            colors.control_border_hovered,
        )
    } else {
        (colors.control_background, colors.control_border)
    };
    ComboBoxFieldStyle {
        background,
        border,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
    }
}

fn resolve_combo_box_option_style(
    colors: &ColorTheme,
    disabled: bool,
    selected: bool,
    hovered: bool,
) -> ComboBoxOptionStyle {
    let background = if disabled {
        colors.control_background_disabled
    } else if hovered {
        colors.item_background_hovered
    } else if selected {
        colors.item_background_selected
    } else {
        colors.popup_background
    };
    ComboBoxOptionStyle {
        background,
        foreground: if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        },
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
    mode: Res<ThemeMode>,
    roots: Query<(Entity, &ComboBoxOptions, Has<InteractionDisabled>), Added<ComboBoxOptions>>,
    hierarchy: ComboBoxHierarchy,
    popups: Query<&Children, With<ComboBoxPopup>>,
    options: Query<(&ComboBoxOption, Has<Selected>)>,
    mut commands: Commands,
) {
    for (root, option_labels, disabled) in &roots {
        commands.entity(root).insert(combo_box_root_node());

        if let Some(field) = hierarchy.field(root) {
            let style = resolve_combo_box_field_style(mode.colors(), disabled, false, false, false);

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
                BackgroundColor(mode.colors().popup_background),
                BorderColor::all(mode.colors().popup_border),
            ));

            // -----------------------------------------------------------------
            // Options
            // -----------------------------------------------------------------

            for &option_entity in popup_children.iter() {
                let Ok((option, selected)) = options.get(option_entity) else {
                    continue;
                };

                let style =
                    resolve_combo_box_option_style(mode.colors(), disabled, selected, false);

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
    fn refresh(&mut self, field: Entity, colors: &ColorTheme) {
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
        let style = resolve_combo_box_field_style(colors, disabled, open, pressed, hovered.0);
        background.0 = style.background;
        *border = BorderColor::all(style.border);
        foreground.0 = ForegroundColor(style.foreground);
    }
    fn refresh_root(&mut self, root: Entity, colors: &ColorTheme) {
        if let Some(field) = self.hierarchy.field(root) {
            self.refresh(field, colors);
        }
    }
}

fn update_combo_box_field_style_changed(
    changed: Query<Entity, (With<ComboBoxField>, Or<(Changed<Hovered>, Added<Pressed>)>)>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for field in &changed {
        styles.refresh(field, mode.colors());
    }
}

fn update_combo_box_field_style_pressed_removed(
    mut removed: RemovedComponents<Pressed>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for field in removed.read() {
        styles.refresh(field, mode.colors());
    }
}

fn update_combo_box_field_style_popup_changed(
    changed: Query<&ChildOf, (With<ComboBoxPopup>, Changed<Visibility>)>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for parent in &changed {
        styles.refresh_root(parent.parent(), mode.colors());
    }
}

fn update_combo_box_field_style_disabled_changed(
    added: Query<Entity, (With<ComboBox>, Added<InteractionDisabled>)>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for root in &added {
        styles.refresh_root(root, mode.colors());
    }
    for root in removed.read() {
        styles.refresh_root(root, mode.colors());
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
    fn refresh(&mut self, option: Entity, colors: &ColorTheme) {
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
        let style = resolve_combo_box_option_style(colors, disabled, selected, hovered.0);
        background.0 = style.background;
        foreground.0 = ForegroundColor(style.foreground);
    }
    fn refresh_root(&mut self, root: Entity, colors: &ColorTheme) {
        let Some(popup) = self.hierarchy.popup(root) else {
            return;
        };
        let Ok(children) = self.children.get(popup) else {
            return;
        };
        let options: Vec<Entity> = children.iter().copied().collect();
        for option in options {
            self.refresh(option, colors);
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
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for option in &changed {
        styles.refresh(option, mode.colors());
    }
}

fn update_combo_box_option_style_selected_removed(
    mut removed: RemovedComponents<Selected>,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for option in removed.read() {
        styles.refresh(option, mode.colors());
    }
}

fn update_combo_box_option_style_disabled_changed(
    added: Query<Entity, (With<ComboBox>, Added<InteractionDisabled>)>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for root in &added {
        styles.refresh_root(root, mode.colors());
    }
    for root in removed.read() {
        styles.refresh_root(root, mode.colors());
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct PopupStyles<'w, 's> {
    hierarchy: ComboBoxHierarchy<'w, 's>,
    popups: Query<
        'w,
        's,
        (&'static mut BackgroundColor, &'static mut BorderColor),
        With<ComboBoxPopup>,
    >,
}

impl PopupStyles<'_, '_> {
    fn refresh_root(&mut self, root: Entity, colors: &ColorTheme) {
        let Some(popup) = self.hierarchy.popup(root) else {
            return;
        };
        if let Ok((mut background, mut border)) = self.popups.get_mut(popup) {
            background.0 = colors.popup_background;
            *border = BorderColor::all(colors.popup_border);
        }
    }
}

fn refresh_combo_box_theme(
    event: On<ThemeChanged>,
    roots: Query<Entity, With<ComboBoxOptions>>,
    mut styles: ParamSet<(FieldStyles, OptionStyles, PopupStyles)>,
) {
    let colors = event.mode.colors();
    for root in &roots {
        styles.p0().refresh_root(root, colors);
    }
    for root in &roots {
        styles.p1().refresh_root(root, colors);
    }
    for root in &roots {
        styles.p2().refresh_root(root, colors);
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

        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(refresh_combo_box_theme);
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

    const TEST_THEME: ColorTheme = ColorTheme {
        foreground: Color::srgb_u8(1, 0, 0),
        foreground_disabled: Color::srgb_u8(2, 0, 0),
        control_background: Color::srgb_u8(3, 0, 0),
        control_background_hovered: Color::srgb_u8(4, 0, 0),
        control_background_pressed: Color::srgb_u8(5, 0, 0),
        control_background_active: Color::srgb_u8(6, 0, 0),
        control_background_disabled: Color::srgb_u8(7, 0, 0),
        control_border: Color::srgb_u8(8, 0, 0),
        control_border_hovered: Color::srgb_u8(9, 0, 0),
        control_border_pressed: Color::srgb_u8(10, 0, 0),
        control_border_active: Color::srgb_u8(11, 0, 0),
        control_border_disabled: Color::srgb_u8(12, 0, 0),
        popup_background: Color::srgb_u8(13, 0, 0),
        popup_border: Color::srgb_u8(14, 0, 0),
        item_background_hovered: Color::srgb_u8(15, 0, 0),
        item_background_selected: Color::srgb_u8(16, 0, 0),
    };

    #[test]
    fn resolves_field_style_with_state_priority() {
        let c = &TEST_THEME;
        for (disabled, open, pressed, hovered, background, border, foreground) in [
            (
                false,
                false,
                false,
                false,
                c.control_background,
                c.control_border,
                c.foreground,
            ),
            (
                false,
                false,
                false,
                true,
                c.control_background_hovered,
                c.control_border_hovered,
                c.foreground,
            ),
            (
                false,
                false,
                true,
                true,
                c.control_background_pressed,
                c.control_border_pressed,
                c.foreground,
            ),
            (
                false,
                true,
                true,
                true,
                c.control_background_active,
                c.control_border_active,
                c.foreground,
            ),
            (
                true,
                true,
                true,
                true,
                c.control_background_disabled,
                c.control_border_disabled,
                c.foreground_disabled,
            ),
        ] {
            assert_eq!(
                resolve_combo_box_field_style(c, disabled, open, pressed, hovered),
                ComboBoxFieldStyle {
                    background,
                    border,
                    foreground
                }
            );
        }
    }

    #[test]
    fn resolves_option_style_with_state_priority() {
        let c = &TEST_THEME;
        for (disabled, selected, hovered, background, foreground) in [
            (false, false, false, c.popup_background, c.foreground),
            (false, true, false, c.item_background_selected, c.foreground),
            (false, true, true, c.item_background_hovered, c.foreground),
            (
                true,
                true,
                true,
                c.control_background_disabled,
                c.foreground_disabled,
            ),
        ] {
            assert_eq!(
                resolve_combo_box_option_style(c, disabled, selected, hovered),
                ComboBoxOptionStyle {
                    background,
                    foreground
                }
            );
        }
    }
}
