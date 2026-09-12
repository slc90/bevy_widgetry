use crate::headless::{ComboBox, ComboBoxField, ComboBoxHierarchy, ComboBoxOption, ComboBoxPopup};
use bevy::prelude::*;
use bevy::ui::Selected;
use bevy::ui_widgets::{Button, ListBox, ListItem};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 每个控件保存自身诊断状态；构造命令全部提交后才检查，避免把暂时未完成的树判为错误。
#[derive(Component, Default)]
pub(crate) struct ComboBoxDiagnostics {
    /// 最近已报告的异常，健康状态为 None。
    reported: Option<HierarchyFault>,
    /// 固定选项列表首次验证成功后的数量，用于发现运行时内部选项丢失。
    option_count: Option<usize>,
}

/// 必需的内部结构被破坏，区别于非控件事件或越界 API 参数。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HierarchyFault {
    Field,
    Popup,
    Visibility,
    Options,
    Selection,
}

/// 在完整帧末检查 Widgetry 根节点，每个异常状态只留下进入与恢复事实。
pub(crate) fn diagnose_combo_boxes(
    mut roots: Query<(Entity, &mut ComboBoxDiagnostics), With<ComboBox>>,
    hierarchy: ComboBoxHierarchy,
    fields: Query<Has<Button>, With<ComboBoxField>>,
    popups: Query<Has<ListBox>, With<ComboBoxPopup>>,
    visibility: Query<&Visibility, With<ComboBoxPopup>>,
    children: Query<&Children>,
    options: Query<(&ComboBoxOption, Has<Selected>, Has<ListItem>)>,
) {
    for (entity, mut diagnostics) in &mut roots {
        let fault = (|| {
            let Ok(root_children) = children.get(entity) else {
                return Some(HierarchyFault::Field);
            };
            if root_children
                .iter()
                .filter(|child| fields.contains(*child))
                .count()
                != 1
            {
                return Some(HierarchyFault::Field);
            }
            if !hierarchy
                .field(entity)
                .is_some_and(|field| fields.get(field) == Ok(true))
            {
                return Some(HierarchyFault::Field);
            }
            if root_children
                .iter()
                .filter(|child| popups.contains(*child))
                .count()
                != 1
            {
                return Some(HierarchyFault::Popup);
            }
            let Some(popup) = hierarchy.popup(entity) else {
                return Some(HierarchyFault::Popup);
            };
            if popups.get(popup) != Ok(true) {
                return Some(HierarchyFault::Popup);
            }
            if visibility.get(popup).is_err() {
                return Some(HierarchyFault::Visibility);
            }
            let Ok(items) = children.get(popup) else {
                return Some(HierarchyFault::Options);
            };
            if diagnostics
                .option_count
                .is_some_and(|count| count != items.len())
            {
                return Some(HierarchyFault::Options);
            }
            let mut selection_count = 0;
            for (index, child) in items.iter().enumerate() {
                let Ok((option, selected, list_item)) = options.get(child) else {
                    return Some(HierarchyFault::Options);
                };
                if option.index != index || !list_item {
                    return Some(HierarchyFault::Options);
                }
                selection_count += usize::from(selected);
            }
            if selection_count != 1 {
                return Some(HierarchyFault::Selection);
            }
            diagnostics.option_count = Some(items.len());
            None
        })();
        if fault != diagnostics.reported {
            match fault {
                Some(error) => {
                    widgetry_error!(?entity, ?error, "ComboBox 内部层级或选择不变量被破坏")
                }
                None => widgetry_info!(?entity, "ComboBox 内部状态恢复正常"),
            }
            diagnostics.reported = fault;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::{ComboBoxPlugin, spawn_headless_combo_box};
    use bevy::ecs::schedule::SingleThreadedExecutor;
    use bevy::ui_widgets::{Button, ListBox, ListItem};
    use bevy_widgetry_test_utils::LogCapture;
    use rstest::rstest;

    // 必需的交互组件被移除时，内部标记仍在也必须报告；持续缺失不重复，补回后恢复。
    #[rstest]
    #[case::field_button(0)]
    #[case::popup_list(1)]
    #[case::option_item(2)]
    fn missing_behavior_component_is_reported(#[case] part: usize) {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins(ComboBoxPlugin)
                .edit_schedule(PostUpdate, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
            let root = spawn_headless_combo_box(&mut app.world_mut().commands(), 2, 0);
            app.update();
            assert_eq!(capture.records().len(), 1);
            let field = app
                .world_mut()
                .query_filtered::<Entity, With<ComboBoxField>>()
                .single(app.world())
                .unwrap();
            let popup = app
                .world_mut()
                .query_filtered::<Entity, With<ComboBoxPopup>>()
                .single(app.world())
                .unwrap();
            let option = app.world().get::<Children>(popup).unwrap()[0];
            match part {
                0 => {
                    app.world_mut().entity_mut(field).remove::<Button>();
                }
                1 => {
                    app.world_mut().entity_mut(popup).remove::<ListBox>();
                }
                _ => {
                    app.world_mut().entity_mut(option).remove::<ListItem>();
                }
            }
            app.update();
            app.update();
            let records = capture.records();
            assert_eq!(records.len(), 2);
            assert_eq!(records[1].level, bevy::log::Level::ERROR);
            assert_eq!(records[1].fields["entity"], format!("{root:?}"));
            match part {
                0 => {
                    app.world_mut().entity_mut(field).insert(Button);
                }
                1 => {
                    app.world_mut().entity_mut(popup).insert(ListBox);
                }
                _ => {
                    app.world_mut().entity_mut(option).insert(ListItem);
                }
            }
            app.update();
            app.update();
            assert_eq!(capture.records().len(), 3);
            assert!(capture.records()[2].fields["message"].contains("恢复"));
        });
    }
}
