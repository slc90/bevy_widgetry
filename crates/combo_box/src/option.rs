use crate::combo_box::WidgetryComboBox;
use crate::popup::ComboBoxPopup;
use bevy::app::Propagate;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy::ui_widgets::ListItem;
use bevy_widgetry_core::{ColorTheme, ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_log::widgetry_error;

/// 固定 list row index；任意内容放在该 ListItem wrapper 的 children 中。
#[derive(Component)]
pub(crate) struct ComboBoxOption {
    /// 对应 root option factory 的零起始位置。
    pub(crate) index: usize,
}

/// option 专属 style 访问，root 是 disabled state 的唯一来源。
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct OptionStyles<'w, 's> {
    /// Popup 到 root 的直接 relationship。
    popups: Query<'w, 's, &'static ChildOf, With<ComboBoxPopup>>,
    /// root 当前的权威 disabled state。
    roots: Query<'w, 's, Has<InteractionDisabled>, With<WidgetryComboBox>>,
    /// wrapper 的交互输入与颜色输出。
    rows: Query<
        'w,
        's,
        (
            Entity,
            &'static ChildOf,
            &'static Hovered,
            Has<Selected>,
            &'static mut BackgroundColor,
            &'static mut Propagate<ForegroundColor>,
        ),
        With<ComboBoxOption>,
    >,
}

/// 保留已有行高和间距；内容 layout 完全由 factory 决定。
pub(crate) fn scene(index: usize, content: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        template(move |_| Ok(ComboBoxOption { index }))
        ListItem Hovered(false)
        { (index == 0).then(|| bsn! { Selected }) }
        Node {
            width: percent(100), height: px(32), align_items: AlignItems::Center,
            padding: UiRect::axes(px(10), px(0)), border_radius: BorderRadius::all(px(4)),
        }
        BackgroundColor
        template(|_| Ok(Propagate(ForegroundColor::default())))
        Children [{content}]
    }
}

/// 新 row、hover 或 selection 加入时重新解析 style。
pub(crate) fn update_changed(
    changed: Query<
        Entity,
        (
            With<ComboBoxOption>,
            Or<(Added<ComboBoxOption>, Changed<Hovered>, Added<Selected>)>,
        ),
    >,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for row in &changed {
        styles.refresh(row, mode.colors());
    }
}

/// 移除 selection 时恢复 hover 或默认背景。
pub(crate) fn update_removed(
    mut removed: RemovedComponents<Selected>,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for row in removed.read() {
        styles.refresh(row, mode.colors());
    }
}

/// root 的 disabled component 增删只刷新所属 list，不给任意 children 挂 disabled component。
pub(crate) fn update_disabled(
    added: Query<Entity, (With<WidgetryComboBox>, Added<InteractionDisabled>)>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mode: Res<ThemeMode>,
    mut styles: OptionStyles,
) {
    for root in added.iter().chain(removed.read()) {
        styles.refresh_root(root, mode.colors());
    }
}

/// theme 通知立即刷新所有 wrapper 的背景与默认传播的 foreground color。
pub(crate) fn refresh_theme(event: On<ThemeChanged>, mut styles: OptionStyles) {
    let roots: Vec<_> = styles.popups.iter().map(ChildOf::parent).collect();
    for root in roots {
        styles.refresh_root(root, event.mode.colors());
    }
}

impl OptionStyles<'_, '_> {
    /// 仅解析真实 option；disabled 优先于 hover，hover 优先于 selected。
    fn refresh(&mut self, row: Entity, colors: &ColorTheme) {
        let Ok((_, parent, hovered, selected, mut background, mut foreground)) =
            self.rows.get_mut(row)
        else {
            return;
        };
        let Ok(root) = self.popups.get(parent.parent()) else {
            widgetry_error!(?row, "ComboBox 选项缺少所属Popup");
            return;
        };
        let Ok(disabled) = self.roots.get(root.parent()) else {
            widgetry_error!(?row, root = ?root.parent(), "ComboBox 选项缺少有效root");
            return;
        };
        background.0 = if disabled {
            colors.control_background_disabled
        } else if hovered.0 {
            colors.item_background_hovered
        } else if selected {
            colors.item_background_selected
        } else {
            colors.popup_background
        };
        foreground.0 = ForegroundColor(if disabled {
            colors.foreground_disabled
        } else {
            colors.foreground
        });
    }

    /// root 据直接 parent relationship 定位同 root 的 row，不缓存可失效的 entity 集合。
    fn refresh_root(&mut self, root: Entity, colors: &ColorTheme) {
        if !self.roots.contains(root) {
            return;
        }
        let rows: Vec<_> = self
            .rows
            .iter()
            .filter_map(|(entity, parent, _, _, _, _)| {
                self.popups
                    .get(parent.parent())
                    .ok()
                    .filter(|parent| parent.parent() == root)
                    .map(|_| entity)
            })
            .collect();
        for row in rows {
            self.refresh(row, colors);
        }
    }
}
