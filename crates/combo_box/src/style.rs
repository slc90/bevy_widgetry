use crate::headless::{
    ComboBox, ComboBoxField, ComboBoxHierarchy, ComboBoxOption, ComboBoxPlugin, ComboBoxPopup,
    spawn_headless_combo_box,
};
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
        AlignItems, BackgroundColor, BorderColor, FlexDirection, GlobalZIndex, InteractionDisabled,
        JustifyContent, Node, PositionType, Pressed, Selected, UiRect, Val, px, widget::Text,
    },
    utils::default,
};
use bevy_widgetry_core::{
    ColorTheme, ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin,
    WidgetryFontPlugin,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 下拉选择根节点的默认逻辑像素宽度。
const COMBO_BOX_WIDTH: f32 = 200.0;

/// 输入区域的逻辑像素高度，同时决定弹层起始位置。
const FIELD_HEIGHT: f32 = 36.0;

/// 每个选项的逻辑像素高度，保持列表命中区域一致。
const OPTION_HEIGHT: f32 = 32.0;

/// 输入区与选项的水平内容留白。
const HORIZONTAL_PADDING: f32 = 10.0;

/// 输入区和弹层共用的逻辑像素边框宽度。
const BORDER_WIDTH: f32 = 1.0;

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

/// 输入区域在当前根状态与指针状态下的完整配色。
#[derive(Debug, Clone, Copy, PartialEq)]
struct ComboBoxFieldStyle {
    /// 状态解析完成后要写入节点的背景色。
    background: Color,
    /// 状态解析完成后要写入节点的边框色。
    border: Color,
    /// 普通状态下文本与图标使用的前景色。
    foreground: Color,
}

/// 选项的背景与前景配色，区分禁用、悬停和选中。
#[derive(Debug, Clone, Copy, PartialEq)]
struct ComboBoxOptionStyle {
    /// 状态解析完成后要写入节点的背景色。
    background: Color,
    /// 普通状态下文本与图标使用的前景色。
    foreground: Color,
}

/// 将根禁用状态、弹层显隐与输入区样式访问集中在同一系统参数中。
#[derive(bevy::ecs::system::SystemParam)]
struct FieldStyles<'w, 's> {
    hierarchy: ComboBoxHierarchy<'w, 's>,
    roots: Query<'w, 's, Has<InteractionDisabled>, With<ComboBox>>,
    popups: Query<'w, 's, &'static Visibility, With<ComboBoxPopup>>,
    fields: FieldStyleQuery<'w, 's>,
}

/// 集中访问选项的根状态与可视数据，供选择、悬停和主题刷新共用。
#[derive(bevy::ecs::system::SystemParam)]
struct OptionStyles<'w, 's> {
    hierarchy: ComboBoxHierarchy<'w, 's>,
    roots: Query<'w, 's, Has<InteractionDisabled>, With<ComboBox>>,
    children: Query<'w, 's, &'static Children>,
    options: OptionStyleQuery<'w, 's>,
}

/// 集中访问弹层容器的可写颜色，供主题刷新使用。
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

/// 装配下拉选择行为、可视层级和主题同步，保持程序化选择与文本一致。
/// 自动提供 App 默认字体；使用内建字体时须先注册 Bevy 资产与文本插件（通常为 DefaultPlugins）。
pub struct StyledComboBoxPlugin;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type FieldStyleQuery<'w, 's> = Query<
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
>;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type ChangedFieldQuery<'w, 's> =
    Query<'w, 's, Entity, (With<ComboBoxField>, Or<(Changed<Hovered>, Added<Pressed>)>)>;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type OptionStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Hovered,
        Has<Selected>,
        &'static mut BackgroundColor,
        &'static mut Propagate<ForegroundColor>,
    ),
    With<ComboBoxOption>,
>;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type ChangedOptionQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        With<ComboBoxOption>,
        Or<(Changed<Hovered>, Changed<Selected>)>,
    ),
>;

/// 按禁用、打开、按压、悬停、普通的优先级解析输入区。
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

/// 禁用覆盖全部交互状态，悬停背景优先于选中背景。
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

/// 为整个选择控件提供固定宽度和容纳弹层的布局根。
fn combo_box_root_node() -> Node {
    Node {
        width: px(COMBO_BOX_WIDTH),
        ..default()
    }
}

/// 将当前选项文本与下拉提示排列在可点击输入区域。
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

/// 将弹层定位在输入区下方，并提供列表边框与纵向布局。
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

/// 为每个选项设置统一高度与水平内边距，保持点击区域一致。
fn combo_box_option_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: px(OPTION_HEIGHT),

        align_items: AlignItems::Center,

        padding: UiRect::axes(px(HORIZONTAL_PADDING), px(0)),

        ..default()
    }
}

/// 创建固定文本选项和主题样式，默认选择首项；需注册 StyledComboBoxPlugin。
/// options 不得为空，否则触发断言；创建后不支持动态修改选项文本。
pub fn spawn_styled_combo_box(commands: &mut Commands, options: Vec<String>) -> Entity {
    assert!(
        !options.is_empty(),
        "StyledComboBox requires at least one option"
    );

    let combo_box = spawn_headless_combo_box(commands, options.len(), 0);

    commands.entity(combo_box).insert(ComboBoxOptions(options));

    combo_box
}

/// 首次加入选项文本时，为已有基础层级添加可视组件和文本子实体。
fn setup_styled_combo_box(
    mode: Res<ThemeMode>,
    roots: Query<(Entity, &ComboBoxOptions, Has<InteractionDisabled>), Added<ComboBoxOptions>>,
    hierarchy: ComboBoxHierarchy,
    popups: Query<&Children, With<ComboBoxPopup>>,
    options: Query<(&ComboBoxOption, Has<Selected>)>,
    mut commands: Commands,
) {
    for (root, option_labels, disabled) in &roots {
        let first_label = &option_labels.0[0];
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
                    field.spawn((ComboBoxFieldText, Text::new(first_label.as_str())));

                    field.spawn((ComboBoxDropdownIcon, Text::new("v")));
                });
        }

        if let Some(popup) = hierarchy.popup(root) {
            let Ok(popup_children) = popups.get(popup) else {
                continue;
            };

            commands.entity(popup).insert((
                combo_box_popup_node(),
                // Popup 必须在上层，避免被其他元素遮挡
                GlobalZIndex(100),
                BackgroundColor(mode.colors().popup_background),
                BorderColor::all(mode.colors().popup_border),
            ));

            for &option_entity in popup_children.iter() {
                let Ok((option, selected)) = options.get(option_entity) else {
                    continue;
                };
                if option.index >= option_labels.0.len() {
                    widgetry_error!(
                        ?root,
                        option_index = option.index,
                        option_count = option_labels.0.len(),
                        "ComboBox 内部选项索引越界"
                    );
                }
                let label = &option_labels.0[option.index];

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
                    .with_child(Text::new(label.as_str()));
            }
        }
    }
}

/// 从实时层级读取弹层可见性，缺失弹层视为未打开。
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

/// 输入区悬停或按压变化时，结合根禁用与弹层状态更新颜色。
fn update_combo_box_field_style_changed(
    changed: ChangedFieldQuery<'_, '_>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for field in &changed {
        styles.refresh(field, mode.colors());
    }
}

/// 按压标记移除后重新解析，恢复仍有效的打开或悬停配色。
fn update_combo_box_field_style_pressed_removed(
    mut removed: RemovedComponents<Pressed>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for field in removed.read() {
        styles.refresh(field, mode.colors());
    }
}

/// 弹层显隐变化时刷新所属输入区的打开状态样式。
fn update_combo_box_field_style_popup_changed(
    changed: Query<&ChildOf, (With<ComboBoxPopup>, Changed<Visibility>)>,
    mode: Res<ThemeMode>,
    mut styles: FieldStyles,
) {
    for parent in &changed {
        styles.refresh_root(parent.parent(), mode.colors());
    }
}

/// 处理根禁用组件的加入与移除，使输入区及时覆盖或恢复交互颜色。
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

/// 选项悬停或选择变更时，结合根禁用状态应用颜色。
fn update_combo_box_option_style_changed(
    changed: ChangedOptionQuery<'_, '_>,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for option in &changed {
        styles.refresh(option, mode.colors());
    }
}

/// 选择组件移除后恢复当前悬停或普通背景。
fn update_combo_box_option_style_selected_removed(
    mut removed: RemovedComponents<Selected>,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for option in removed.read() {
        styles.refresh(option, mode.colors());
    }
}

/// 根禁用状态改变时刷新其所有选项，保持整个控件的禁用外观一致。
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

/// 一次主题通知中刷新输入区、弹层和全部选项，不改变选择与显隐状态。
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

        if option.index >= option_labels.0.len() {
            widgetry_error!(
                ?root,
                option_index = option.index,
                option_count = option_labels.0.len(),
                "ComboBox 内部选项索引越界"
            );
        }
        let label = &option_labels.0[option.index];

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

impl FieldStyles<'_, '_> {
    /// 根据当前 ECS 状态解析完整样式。
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

    /// 从根实体定位内部可视节点并刷新，缺失层级时无需操作。
    fn refresh_root(&mut self, root: Entity, colors: &ColorTheme) {
        if let Some(field) = self.hierarchy.field(root) {
            self.refresh(field, colors);
        }
    }
}

impl OptionStyles<'_, '_> {
    /// 从当前层级和交互状态重新计算该实体的完整配色。
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

    /// 从根实体定位内部可视节点并刷新，缺失层级时无需操作。
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

impl PopupStyles<'_, '_> {
    /// 从根实体定位内部可视节点并刷新，缺失层级时无需操作。
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

impl Plugin for StyledComboBoxPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ComboBoxPlugin>() {
            app.add_plugins(ComboBoxPlugin);
        }

        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }

        if !app.is_plugin_added::<WidgetryFontPlugin>() {
            app.add_plugins(WidgetryFontPlugin);
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
        widgetry_info!("StyledComboBoxPlugin 注册完成");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::text::FontSource;
    use bevy::{ecs::system::RunSystemOnce, log::tracing::Level};
    use bevy_widgetry_core::WidgetryAppExt;
    use bevy_widgetry_test_utils::LogCapture;
    use rstest::rstest;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    // 内部索引越界时，构造和选择同步都必须先记录定位信息，再保留索引访问的 panic。
    #[rstest]
    #[case::setup(false)]
    #[case::selection_sync(true)]
    fn invalid_internal_index_panics(#[case] materialized: bool) {
        let mut app = App::new();
        app.set_default_font(FontSource::Monospace);
        app.add_plugins(StyledComboBoxPlugin);
        let root = spawn_styled_combo_box(&mut app.world_mut().commands(), vec!["A".into()]);
        app.world_mut().flush();
        if materialized {
            app.update();
        }
        let option = app
            .world_mut()
            .query_filtered::<Entity, With<ComboBoxOption>>()
            .single(app.world())
            .unwrap();
        app.world_mut()
            .get_mut::<ComboBoxOption>(option)
            .unwrap()
            .index = usize::MAX;
        app.world_mut().entity_mut(option).insert(Selected);
        let capture = LogCapture::default();
        let result = catch_unwind(AssertUnwindSafe(|| {
            capture.run(|| {
                if materialized {
                    app.world_mut()
                        .run_system_once(update_combo_box_field_text)
                        .unwrap();
                } else {
                    app.world_mut()
                        .run_system_once(setup_styled_combo_box)
                        .unwrap();
                }
            });
        }));
        let panic = result.expect_err("内部索引越界必须 panic");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(message.contains("index out of bounds"));
        let records = capture.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, Level::ERROR);
        assert_eq!(records[0].fields["message"], "ComboBox 内部选项索引越界");
        assert_eq!(records[0].fields["root"], format!("{root:?}"));
        assert_eq!(records[0].fields["option_index"], usize::MAX.to_string());
        assert_eq!(records[0].fields["option_count"], "1");
    }

    const TEST_THEME: ColorTheme = ColorTheme {
        window_background: Color::BLACK,
        window_border: Color::WHITE,
        title_bar_border: Color::WHITE,
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
        text_selection: Color::srgb_u8(52, 92, 140),
        text_selection_unfocused: Color::srgb_u8(65, 70, 78),
    };

    // 使用可区分的状态颜色逐项组合，验证输入区背景、边框和前景色采用同一优先级。
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

    // 组合禁用、选中与悬停状态，验证背景优先级和禁用前景色独立正确。
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
