use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

use crate::combo_box::WidgetryComboBox;
use crate::field::ComboBoxField;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{
    Activate,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};
use bevy_widgetry_core::z_index;
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListView, WidgetryListViewItem, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_log::widgetry_error;
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxPopup;

pub(crate) fn scene<T: Send + Sync + 'static>(
    source: Entity,
    item_height: f32,
    renderer: WidgetryListViewRenderer<T>,
) -> impl Scene {
    // 将复合 ListView 的 Scene type erase，限制嵌套 Gallery Scene 展开时的 stack 占用。
    let list: Box<dyn Scene> = Box::new(bsn! {
        @WidgetryListView::<T> { @source: source, @item_height: item_height, @renderer: {renderer} }
        TabIndex(-1)
        Node { width: percent(100), height: percent(100), border: UiRect::ZERO }
    });
    bsn! {
        ComboBoxPopup Visibility::Hidden GlobalZIndex({z_index::POPUP})
        Popover {
            positions: {vec![
                PopoverPlacement { side: PopoverSide::Bottom, align: PopoverAlign::Start, gap: 0.0 },
                PopoverPlacement { side: PopoverSide::Top, align: PopoverAlign::Start, gap: 0.0 },
            ]},
            window_margin: 8.0,
        }
        template(|context| Ok(BackgroundColor(context.resource::<WidgetryThemeMode>().colors().combo_box.popup.normal.background)))
        template(|context| Ok(BorderColor::all(context.resource::<WidgetryThemeMode>().colors().combo_box.popup.normal.border)))
        Node {
            position_type: PositionType::Absolute,
            width: percent(100), height: px(2), box_sizing: BoxSizing::BorderBox,
            flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch,
            border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(4)),
        }
        Children [({list})]
    }
}

pub(crate) fn handle_row_click<T: Send + Sync + 'static>(
    event: On<Pointer<Click>>,
    lists: Query<&ChildOf, With<WidgetryListView<T>>>,
    popups: Query<&ChildOf, With<ComboBoxPopup>>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let list = event.entity;
    let Ok(parent) = lists.get(list) else {
        return;
    };
    let popup = parent.parent();
    let Ok(parent) = popups.get(popup) else {
        return;
    };
    let root = parent.parent();
    if !roots.contains(root) {
        return;
    }
    let target = event.original_event_target();
    commands.queue(move |world: &mut World| {
        if world.get::<InteractionDisabled>(root).is_some()
            || world.get::<InteractionDisabled>(list).is_some()
            || world.get::<Visibility>(popup) != Some(&Visibility::Visible)
        {
            return;
        }
        let mut entity = target;
        let mut item = None;
        while entity != list {
            // 最近 ListView 是 ownership 边界，嵌套或 foreign rows 不能冒充本列表。
            if world.get::<WidgetryListViewState>(entity).is_some() {
                return;
            }
            if item.is_none() {
                item = world.get::<WidgetryListViewItem>(entity).copied();
            }
            let Some(parent) = world.get::<ChildOf>(entity) else {
                return;
            };
            entity = parent.parent();
        }
        let Some(item) = item else {
            return;
        };
        let Some(view) = world.get::<WidgetryListView<T>>(list) else {
            return;
        };
        let Some(model) = world.get::<WidgetryListModel<T>>(view.source()) else {
            return;
        };
        let Some(index) = model.index_of(item.id) else {
            return;
        };
        if model.is_disabled(index) == Some(false)
            && let Some(mut visibility) = world.get_mut::<Visibility>(popup)
        {
            *visibility = Visibility::Hidden;
        }
    });
}

pub(crate) fn handle_reselection<T: Send + Sync + 'static>(
    event: On<FocusedInput<KeyboardInput>>,
    lists: Query<&ChildOf, With<WidgetryListView<T>>>,
    popups: Query<&ChildOf, With<ComboBoxPopup>>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    let list = event.focused_entity;
    if event.input.state != ButtonState::Pressed
        || !matches!(event.input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        return;
    }
    let Ok(parent) = lists.get(list) else {
        return;
    };
    let popup = parent.parent();
    let Ok(parent) = popups.get(popup) else {
        return;
    };
    let root = parent.parent();
    if !roots.contains(root) {
        return;
    }
    // 与 ListView queued navigation 顺序执行，避免读取同帧前一个按键执行前的 active。
    commands.queue(move |world: &mut World| {
        if world.get::<Visibility>(popup) != Some(&Visibility::Visible)
            || world.get::<InteractionDisabled>(root).is_some()
            || world.get::<InteractionDisabled>(list).is_some()
            || world
                .get_resource::<InputFocus>()
                .is_none_or(|focus| focus.get() != Some(list))
        {
            return;
        }
        let Some(state) = world.get::<WidgetryListViewState>(list) else {
            return;
        };
        let Some(active) = state
            .active
            .filter(|active| Some(*active) == state.selected)
        else {
            return;
        };
        let Some(view) = world.get::<WidgetryListView<T>>(list) else {
            return;
        };
        let Some(model) = world.get::<WidgetryListModel<T>>(view.source()) else {
            return;
        };
        if !model
            .index_of(active)
            .is_some_and(|index| model.is_disabled(index) == Some(false))
        {
            return;
        }
        if let Some(mut visibility) = world.get_mut::<Visibility>(popup) {
            *visibility = Visibility::Hidden;
        }
        if let Some(mut focus) = world.get_resource_mut::<InputFocus>() {
            focus.clear();
        }
    });
}

pub(crate) fn handle_escape<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    lists: Query<&ChildOf, With<WidgetryListView<T>>>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
    roots: Query<&Children, With<WidgetryComboBox<T>>>,
    fields: Query<(), With<ComboBoxField>>,
    mut focus: ResMut<InputFocus>,
) -> Result<(), BevyError> {
    let list = event.focused_entity;
    if event.input.key_code != KeyCode::Escape
        || event.input.state != ButtonState::Pressed
        || focus.get() != Some(list)
    {
        return Ok(());
    }
    let Ok(parent) = lists.get(list) else {
        return Ok(());
    };
    let Ok((parent, mut visibility)) = popups.get_mut(parent.parent()) else {
        return Ok(());
    };
    let Ok(children) = roots.get(parent.parent()) else {
        return Ok(());
    };
    if *visibility != Visibility::Visible {
        return Ok(());
    }
    let Some(field) = children.iter().find(|&child| fields.contains(child)) else {
        widgetry_error!(root = ?parent.parent(), "ComboBox 缺少 Field");
        return Err(BevyError::error(
            "ComboBox required popup structure missing",
        ));
    };
    event.propagate(false);
    *visibility = Visibility::Hidden;
    focus.set(field, FocusCause::Navigated);
    Ok(())
}

pub(crate) fn clear_hidden_focus<T: Send + Sync + 'static>(
    lists: Query<&ChildOf, With<WidgetryListView<T>>>,
    popups: Query<(&ChildOf, &Visibility), With<ComboBoxPopup>>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    focus: Option<ResMut<InputFocus>>,
) {
    let Some(mut focus) = focus else {
        return;
    };
    let Some(focused) = focus.get() else {
        return;
    };
    let Ok(parent) = lists.get(focused) else {
        return;
    };
    let Ok((parent, visibility)) = popups.get(parent.parent()) else {
        return;
    };
    if roots.contains(parent.parent()) && *visibility != Visibility::Visible {
        focus.clear();
    }
}

pub(crate) fn sync_geometry<T: Send + Sync + 'static>(
    roots: Query<(&WidgetryComboBox<T>, &Children)>,
    models: Query<&WidgetryListModel<T>>,
    mut popups: Query<(&mut Node, &mut Visibility), With<ComboBoxPopup>>,
) {
    for (combo, children) in &roots {
        let Ok(model) = models.get(combo.source()) else {
            // source 缺失时内部 ListView 已负责校验并传播错误。
            // 此处跳过高度更新，避免同一失败重复诊断。
            continue;
        };
        for child in children.iter() {
            if let Ok((mut node, mut visibility)) = popups.get_mut(child) {
                let height = px(model.len().min(combo.max_visible_items()) as f32
                    * combo.item_height()
                    + 2.0);
                if node.height != height {
                    node.height = height;
                }
                if model.is_empty() && *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}

pub(crate) fn handle_field_activate<T: Send + Sync + 'static>(
    event: On<Activate>,
    fields: Query<&ChildOf, With<ComboBoxField>>,
    roots: Query<(&WidgetryComboBox<T>, &Children, Has<InteractionDisabled>)>,
    models: Query<&WidgetryListModel<T>>,
    lists: Query<(), With<WidgetryListView<T>>>,
    mut popups: Query<(&Children, &mut Visibility), With<ComboBoxPopup>>,
    mut focus: Option<ResMut<InputFocus>>,
) -> Result<(), BevyError> {
    let Ok(parent) = fields.get(event.entity) else {
        return Ok(());
    };
    let Ok((combo, children, disabled)) = roots.get(parent.parent()) else {
        return Ok(());
    };
    if disabled
        || !models
            .get(combo.source())
            .is_ok_and(|model| !model.is_empty())
    {
        return Ok(());
    }
    let Some(popup) = children.iter().find(|&child| popups.contains(child)) else {
        widgetry_error!(root = ?parent.parent(), "ComboBox 缺少Popup");
        return Err(BevyError::error(
            "ComboBox required popup structure missing",
        ));
    };
    if let Ok((children, mut visibility)) = popups.get_mut(popup) {
        let Some(list) = children.iter().find(|&child| lists.contains(child)) else {
            widgetry_error!(root = ?parent.parent(), ?popup, "ComboBox 缺少内部 ListView");
            return Err(BevyError::error(
                "ComboBox required popup structure missing",
            ));
        };
        *visibility = if *visibility == Visibility::Hidden {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if *visibility == Visibility::Visible
            && let Some(ref mut focus) = focus
        {
            focus.set(list, FocusCause::Navigated);
        }
    }
    Ok(())
}

pub(crate) fn handle_outside_click(
    event: On<Pointer<Click>>,
    parents: Query<&ChildOf>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
) {
    let target = event.original_event_target();
    for (parent, mut visibility) in &mut popups {
        if *visibility == Visibility::Visible
            && target != parent.parent()
            && !parents
                .iter_ancestors(target)
                .any(|ancestor| ancestor == parent.parent())
        {
            *visibility = Visibility::Hidden;
        }
    }
}

pub(crate) fn refresh_theme(
    event: On<WidgetryThemeChanged>,
    mut popups: Query<(&mut BackgroundColor, &mut BorderColor), With<ComboBoxPopup>>,
) {
    for (mut background, mut border) in &mut popups {
        background.0 = event.mode.colors().combo_box.popup.normal.background;
        *border = BorderColor::all(event.mode.colors().combo_box.popup.normal.border);
    }
}
