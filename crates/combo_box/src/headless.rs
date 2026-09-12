use crate::diagnostics::{ComboBoxDiagnostics, diagnose_combo_boxes};
use bevy::{
    app::{App, Plugin},
    camera::visibility::Visibility,
    ecs::{
        component::Component,
        entity::Entity,
        event::EntityEvent,
        hierarchy::{ChildOf, Children},
        lifecycle::Add,
        observer::On,
        query::{Has, With},
        system::{Commands, Query},
    },
    picking::events::{Click, Pointer},
    ui::{InteractionDisabled, Selected},
    ui_widgets::{Activate, Button, ListBox, ListItem, ValueChange},
};
use bevy_widgetry_log::widgetry_info;

/// Headless ComboBox 的根组件。
///
/// 挂载这个组件的 Entity 代表整个 ComboBox。
#[derive(Component, Debug, Default)]
#[require(ComboBoxDiagnostics)]
pub struct ComboBox;

/// ComboBox 内部的 Field。
#[derive(Component, Debug, Default)]
pub(crate) struct ComboBoxField;

/// ComboBox 内部的 Popup。
#[derive(Component, Debug, Default)]
pub(crate) struct ComboBoxPopup;

/// ComboBox 内部的一个可选项。
#[derive(Component, Debug)]
pub(crate) struct ComboBoxOption {
    /// 选项在固定列表中的零起始位置。
    pub(crate) index: usize,
}

/// 查询当前固定的 ComboBox 层级，避免缓存已失效的内部实体。
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ComboBoxHierarchy<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    fields: Query<'w, 's, (), With<ComboBoxField>>,
    popups: Query<'w, 's, (), With<ComboBoxPopup>>,
}

/// 注册展开、关闭、选择及禁用观察者；用户指针交互还需 Bevy 的控件插件。
pub struct ComboBoxPlugin;

/// 程序化切换选择：禁用状态下仍有效；越界时保持原值，不发出 ValueChange。
#[derive(EntityEvent, Debug)]
pub struct SetComboBoxSelected {
    #[event_target]
    /// 接收该实体事件的控件根实体。
    pub entity: Entity,
    /// 目标选项的零起始索引，超出当前选项范围时忽略请求。
    pub selected: usize,
}

/// 对同一弹层的选项保持唯一 Selected，仅提交实际需要的组件变更。
fn set_selected_option(
    commands: &mut Commands,
    children: &Children,
    selected_option: Entity,
    options: &Query<(Entity, Has<Selected>), With<ComboBoxOption>>,
) {
    for &child in children.iter() {
        let Ok((entity, selected)) = options.get(child) else {
            continue;
        };
        if entity == selected_option && !selected {
            commands.entity(entity).insert(Selected);
        } else if entity != selected_option && selected {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

/// 忽略禁用控件，对有效输入区域切换弹层显隐。
fn handle_combo_box_field_activate(
    event: On<Activate>,
    q_field: Query<&ChildOf, With<ComboBoxField>>,
    q_combo_box: Query<Has<InteractionDisabled>, With<ComboBox>>,
    hierarchy: ComboBoxHierarchy,
    mut q_popup: Query<&mut Visibility, With<ComboBoxPopup>>,
) {
    let Ok(field_parent) = q_field.get(event.entity) else {
        return;
    };

    let combo_box = field_parent.parent();

    let Ok(disabled) = q_combo_box.get(combo_box) else {
        return;
    };

    if disabled {
        return;
    }

    if let Some(popup) = hierarchy.popup(combo_box)
        && let Ok(mut visibility) = q_popup.get_mut(popup)
    {
        *visibility = if *visibility == Visibility::Hidden {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 验证选项归属后提交用户选择、关闭弹层并转发索引事件。
fn handle_combo_box_value_change(
    event: On<ValueChange<Entity>>,
    hierarchy: ComboBoxHierarchy,
    q_combo_box: Query<Has<InteractionDisabled>, With<ComboBox>>,
    q_option: Query<(&ComboBoxOption, &ChildOf)>,
    q_selected: Query<(Entity, Has<Selected>), With<ComboBoxOption>>,
    mut q_visibility: Query<&mut Visibility, With<ComboBoxPopup>>,
    mut commands: Commands,
) {
    // 只处理属于 ComboBox 的 ListBox
    if !hierarchy.popups.contains(event.source) {
        return;
    }
    let Ok(popup_parent) = hierarchy.parents.get(event.source) else {
        return;
    };

    let combo_box = popup_parent.parent();

    let Ok(disabled) = q_combo_box.get(combo_box) else {
        return;
    };

    if disabled {
        return;
    }

    let Ok((selected_option, option_parent)) = q_option.get(event.value) else {
        return;
    };

    if option_parent.parent() != event.source {
        return;
    }

    if let Ok(children) = hierarchy.children.get(event.source) {
        set_selected_option(&mut commands, children, event.value, &q_selected);
    }

    // 关闭 Popup
    if let Ok(mut visibility) = q_visibility.get_mut(event.source) {
        *visibility = Visibility::Hidden;
    }

    // 转换成 ComboBox 自己的语义事件
    commands.trigger(ValueChange::<usize> {
        source: combo_box,
        value: selected_option.index,
        is_final: event.is_final,
    });
}

/// 创建固定选项数量的选择树，初始弹层隐藏。需注册 ComboBoxPlugin。
/// option_count 必须大于零，selected 必须小于选项数量，否则触发断言。
pub fn spawn_headless_combo_box(
    commands: &mut Commands,
    option_count: usize,
    selected: usize,
) -> Entity {
    assert!(option_count > 0);
    assert!(selected < option_count);

    let mut combo_box = commands.spawn(ComboBox);

    combo_box.with_children(|root| {
        root.spawn((ComboBoxField, Button));

        root.spawn((ComboBoxPopup, ListBox, Visibility::Hidden))
            .with_children(|popup| {
                for index in 0..option_count {
                    let mut option = popup.spawn((ComboBoxOption { index }, ListItem));

                    if index == selected {
                        option.insert(Selected);
                    }
                }
            });
    });

    combo_box.id()
}

/// 按原始指针目标判断层级归属，关闭目标控件之外的可见弹层。
fn handle_combo_box_outside_click(
    event: On<Pointer<Click>>,
    q_parents: Query<&ChildOf>,
    mut q_popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
) {
    let target = event.original_event_target();

    for (popup_parent, mut visibility) in &mut q_popups {
        if *visibility != Visibility::Visible {
            continue;
        }

        let combo_box = popup_parent.parent();

        let inside = target == combo_box
            || q_parents
                .iter_ancestors(target)
                .any(|ancestor| ancestor == combo_box);

        if !inside {
            *visibility = Visibility::Hidden;
        }
    }
}

/// 禁用状态加入时关闭弹层，防止继续进行用户选择。
fn handle_combo_box_disabled(
    event: On<Add, InteractionDisabled>,
    roots: Query<(), With<ComboBox>>,
    hierarchy: ComboBoxHierarchy,
    mut popups: Query<&mut Visibility, With<ComboBoxPopup>>,
) {
    if !roots.contains(event.entity) {
        return;
    }
    if let Some(popup) = hierarchy.popup(event.entity)
        && let Ok(mut visibility) = popups.get_mut(popup)
    {
        *visibility = Visibility::Hidden;
    }
}

/// 验证目标根实体和索引后更新选择，不产生用户交互副作用。
fn handle_set_combo_box_selected(
    event: On<SetComboBoxSelected>,
    roots: Query<(), With<ComboBox>>,
    hierarchy: ComboBoxHierarchy,
    children: Query<&Children>,
    options: Query<&ComboBoxOption>,
    selected: Query<(Entity, Has<Selected>), With<ComboBoxOption>>,
    mut commands: Commands,
) {
    if !roots.contains(event.entity) {
        return;
    }
    let Some(popup) = hierarchy.popup(event.entity) else {
        return;
    };
    let Ok(children) = children.get(popup) else {
        return;
    };
    // 先验证再修改；程序化选择不触发用户交互事件。
    let Some(target) = children.iter().copied().find(|&entity| {
        options
            .get(entity)
            .is_ok_and(|option| option.index == event.selected)
    }) else {
        return;
    };
    set_selected_option(&mut commands, children, target, &selected);
}

impl ComboBoxHierarchy<'_, '_> {
    /// 从根实体的直接子节点查找输入区域，层级不完整时返回 None。
    pub(crate) fn field(&self, root: Entity) -> Option<Entity> {
        self.children
            .get(root)
            .ok()?
            .iter()
            .copied()
            .find(|&child| self.fields.contains(child))
    }

    /// 从根实体的直接子节点查找弹层，层级不完整时返回 None。
    pub(crate) fn popup(&self, root: Entity) -> Option<Entity> {
        self.children
            .get(root)
            .ok()?
            .iter()
            .copied()
            .find(|&child| self.popups.contains(child))
    }

    /// 沿选项与弹层的父关系查找所属控件，拒绝不属于弹层的实体。
    pub(crate) fn root_from_option(&self, option: Entity) -> Option<Entity> {
        let popup = self.parents.get(option).ok()?.parent();
        self.popups.get(popup).ok()?;
        Some(self.parents.get(popup).ok()?.parent())
    }
}

impl Plugin for ComboBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_combo_box_field_activate)
            .add_observer(handle_combo_box_value_change)
            .add_observer(handle_combo_box_outside_click)
            .add_observer(handle_combo_box_disabled)
            .add_observer(handle_set_combo_box_selected);
        app.add_systems(bevy::app::PostUpdate, diagnose_combo_boxes);
        widgetry_info!("ComboBoxPlugin 注册完成");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        app::{App, Startup},
        ecs::{
            entity::Entity, hierarchy::ChildOf, query::With, resource::Resource, system::ResMut,
        },
        ui::Selected,
    };
    use bevy_widgetry_test_utils::LogCapture;
    use bevy_widgetry_test_utils::primary_click;

    // 已构造控件丢失 Popup 时只报告异常边沿，修复后报告恢复，再破坏时重新报告。
    #[test]
    fn missing_popup_logs_only_state_edges() {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins(ComboBoxPlugin)
                .edit_schedule(bevy::app::PostUpdate, |s| {
                    s.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
                });
            let root = spawn_headless_combo_box(&mut app.world_mut().commands(), 3, 0);
            app.update();
            let popup = app
                .world_mut()
                .query_filtered::<Entity, With<ComboBoxPopup>>()
                .single(app.world())
                .unwrap();
            app.world_mut().entity_mut(popup).remove::<ComboBoxPopup>();
            app.update();
            app.update();
            assert_eq!(
                capture
                    .records()
                    .iter()
                    .filter(|r| r.level == bevy::log::Level::ERROR)
                    .count(),
                1
            );
            app.world_mut().entity_mut(popup).insert(ComboBoxPopup);
            app.update();
            app.update();
            assert_eq!(
                capture
                    .records()
                    .iter()
                    .filter(|r| r.fields["message"].contains("恢复"))
                    .count(),
                1
            );
            app.world_mut().entity_mut(popup).remove::<ComboBoxPopup>();
            app.update();
            assert_eq!(
                capture
                    .records()
                    .iter()
                    .filter(|r| r.level == bevy::log::Level::ERROR)
                    .count(),
                2
            );
            assert!(
                capture
                    .records()
                    .iter()
                    .filter(|r| r.level == bevy::log::Level::ERROR)
                    .all(|r| r.fields["entity"].contains(&format!("{root:?}")))
            );
        });
    }

    #[derive(Resource, Default)]
    struct ReceivedValue {
        source: Option<Entity>,
        value: Option<usize>,
    }

    fn record_combo_box_value_change(
        event: On<ValueChange<usize>>,
        mut received: ResMut<ReceivedValue>,
    ) {
        received.source = Some(event.source);
        received.value = Some(event.value);
    }

    fn spawn_test_combo_box(mut commands: Commands) {
        spawn_headless_combo_box(&mut commands, 3, 1);
    }

    #[derive(Resource)]
    struct TestComboBoxes {
        a: Entity,
        b: Entity,
    }

    fn spawn_two_test_combo_boxes(mut commands: Commands) {
        let a = spawn_headless_combo_box(&mut commands, 3, 1);
        let b = spawn_headless_combo_box(&mut commands, 3, 1);

        commands.insert_resource(TestComboBoxes { a, b });
    }

    // 构造三个选项并指定中间项，验证私有层级标记、父关系与唯一选择一致。
    #[test]
    fn spawn_combo_box_should_build_expected_structure() {
        let mut app = App::new();

        app.add_systems(Startup, spawn_test_combo_box);
        app.update();

        let world = app.world_mut();

        // 1. 只有一个 ComboBox root
        let combo_boxes: Vec<Entity> = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();
            query.iter(world).collect()
        };

        assert_eq!(combo_boxes.len(), 1);

        let combo_box = combo_boxes[0];

        // 2. 有一个 Field，并且它是 ComboBox 的直接 child
        let fields: Vec<Entity> = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxField>>();
            query.iter(world).collect()
        };

        assert_eq!(fields.len(), 1);
        assert_eq!(world.get::<ChildOf>(fields[0]).unwrap().parent(), combo_box);

        // 3. 有一个 Popup，并且它也是 ComboBox 的直接 child
        let popups: Vec<Entity> = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();
            query.iter(world).collect()
        };

        assert_eq!(popups.len(), 1);

        let popup = popups[0];

        assert_eq!(world.get::<ChildOf>(popup).unwrap().parent(), combo_box);

        // 4. Popup 下有 3 个 Option
        let mut options: Vec<(Entity, usize)> = {
            let mut query = world.query::<(Entity, &ComboBoxOption, &ChildOf)>();

            query
                .iter(world)
                .filter(|(_, _, parent)| parent.parent() == popup)
                .map(|(entity, option, _)| (entity, option.index))
                .collect()
        };

        options.sort_by_key(|(_, index)| *index);

        assert_eq!(options.len(), 3);

        // 5. index = 1 的 Option 是 Selected
        for (entity, index) in options {
            assert_eq!(world.get::<Selected>(entity).is_some(), index == 1);
        }
    }

    // 连续激活同一输入区域，验证弹层可重复打开和关闭。
    #[test]
    fn field_activate_should_toggle_popup_visibility() {
        let mut app = App::new();
        app.add_plugins(ComboBoxPlugin);
        app.add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let field = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxField>>();

            query.single(world).unwrap()
        };

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        // 初始状态是关闭
        assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);

        // 第一次 Activate：打开
        world.trigger(Activate { entity: field });

        assert_eq!(
            *world.get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );

        // 第二次 Activate：关闭
        world.trigger(Activate { entity: field });

        assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);
    }

    // 从内部列表发送选择事件，验证选择、弹层状态和外部索引通知保持一致。
    #[test]
    fn option_value_change_should_update_selection_and_close_popup() {
        let mut app = App::new();

        app.init_resource::<ReceivedValue>()
            .add_plugins(ComboBoxPlugin)
            .add_observer(record_combo_box_value_change)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        let option_2 = {
            let mut query = world.query::<(Entity, &ComboBoxOption)>();

            query
                .iter(world)
                .find(|(_, option)| option.index == 2)
                .map(|(entity, _)| entity)
                .unwrap()
        };

        // 模拟 Popup 当前已经打开
        *world.get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

        // 模拟内部 ListBox 告诉我们：
        // “现在选择了 option_2”
        world.trigger(ValueChange::<Entity> {
            source: popup,
            value: option_2,
            is_final: true,
        });

        world.flush();

        // 1. index = 2 现在应该是 Selected
        let mut query = world.query::<(Entity, &ComboBoxOption, Has<Selected>)>();

        for (_, option, selected) in query.iter(world) {
            assert_eq!(selected, option.index == 2);
        }

        // 2. Popup 应该关闭
        assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);

        // 3. ComboBox 应该对外发 ValueChange<usize>
        let received = world.resource::<ReceivedValue>();

        assert_eq!(received.source, Some(combo_box));
        assert_eq!(received.value, Some(2));
    }

    // 弹层已打开时点击无关实体，验证外部点击会关闭它。
    #[test]
    fn outside_click_should_close_popup() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        let outside = world.spawn_empty().id();

        // 先模拟 Popup 已经打开
        *world.get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

        // 点击完全无关的 Entity
        world.trigger(primary_click(outside));

        assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);
    }

    // 弹层已打开时点击其选项，验证祖先归属判断不会将内部点击误判为外部。
    #[test]
    fn click_inside_combo_box_should_not_close_popup() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        let option = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxOption>>();

            query.iter(world).next().unwrap()
        };

        // Popup 当前已经打开
        *world.get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

        // 点击 Popup 内部的一个 Option
        world.trigger(primary_click(option));

        assert_eq!(
            *world.get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
    }

    // 点击根实体自身，验证无需祖先匹配也能识别控件内部点击。
    #[test]
    fn click_combo_box_root_should_not_close_popup() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        *world.get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

        world.trigger(primary_click(combo_box));

        assert_eq!(
            *world.get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
    }

    // 禁用根实体后激活内部输入区，验证不会绕过根级交互限制。
    #[test]
    fn disabled_combo_box_should_not_open() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        let field = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxField>>();

            query.single(world).unwrap()
        };

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        // 整个 ComboBox disabled
        world.entity_mut(combo_box).insert(InteractionDisabled);

        // 尝试激活内部 Field
        world.trigger(Activate { entity: field });

        // Popup 仍然不能打开
        assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);
    }

    // 禁用后伪造内部列表通知，验证选择与外部通知均保持不变。
    #[test]
    fn disabled_combo_box_should_ignore_listbox_value_change() {
        let mut app = App::new();

        app.init_resource::<ReceivedValue>()
            .add_plugins(ComboBoxPlugin)
            .add_observer(record_combo_box_value_change)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        let option_2 = {
            let mut query = world.query::<(Entity, &ComboBoxOption)>();

            query
                .iter(world)
                .find(|(_, option)| option.index == 2)
                .map(|(entity, _)| entity)
                .unwrap()
        };

        world.entity_mut(combo_box).insert(InteractionDisabled);

        world.trigger(ValueChange::<Entity> {
            source: popup,
            value: option_2,
            is_final: true,
        });

        world.flush();

        // 原来的 index = 1 仍然保持选中
        let mut query = world.query::<(&ComboBoxOption, Has<Selected>)>();

        for (option, selected) in query.iter(world) {
            assert_eq!(selected, option.index == 1);
        }

        // 没有对外产生新的 value
        let received = world.resource::<ReceivedValue>();

        assert_eq!(received.source, None);
        assert_eq!(received.value, None);
    }

    // 对打开中的控件添加禁用组件，验证生命周期观察者立即关闭弹层。
    #[test]
    fn disabling_open_combo_box_should_close_popup() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        let popup = {
            let mut query = world.query_filtered::<Entity, With<ComboBoxPopup>>();

            query.single(world).unwrap()
        };

        // 先打开
        *world.get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

        // 然后 disabled
        world.entity_mut(combo_box).insert(InteractionDisabled);

        assert_eq!(*world.get::<Visibility>(popup).unwrap(), Visibility::Hidden);
    }

    // 禁用控件仍接受程序化选择，验证用户交互限制不阻止数据赋值。
    #[test]
    fn programmatic_selection_should_work_when_disabled() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        world.entity_mut(combo_box).insert(InteractionDisabled);

        world.trigger(SetComboBoxSelected {
            entity: combo_box,
            selected: 2,
        });

        world.flush();

        let mut query = world.query::<(&ComboBoxOption, Has<Selected>)>();

        for (option, selected) in query.iter(world) {
            assert_eq!(selected, option.index == 2);
        }
    }

    // 传入越界索引，验证先验证再修改的逻辑保留原有唯一选择。
    #[test]
    fn invalid_programmatic_selection_should_keep_current_selection() {
        let mut app = App::new();

        app.add_plugins(ComboBoxPlugin)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        world.trigger(SetComboBoxSelected {
            entity: combo_box,
            selected: 999,
        });

        world.flush();

        let mut query = world.query::<(&ComboBoxOption, Has<Selected>)>();

        for (option, selected) in query.iter(world) {
            assert_eq!(selected, option.index == 1);
        }
    }

    // 同时记录内部和外部选择事件，验证程序化更新只改状态且不关闭弹层。
    #[test]
    fn programmatic_selection_should_not_emit_value_change() {
        let mut app = App::new();

        app.init_resource::<ReceivedValue>()
            .add_plugins(ComboBoxPlugin)
            .add_observer(record_combo_box_value_change)
            .add_systems(Startup, spawn_test_combo_box);

        app.update();

        #[derive(Resource, Default)]
        struct ListBoxEvents(usize);
        app.init_resource::<ListBoxEvents>();
        app.add_observer(
            |_: On<ValueChange<Entity>>, mut count: ResMut<ListBoxEvents>| {
                count.0 += 1;
            },
        );
        let world = app.world_mut();

        let combo_box = {
            let mut query = world.query_filtered::<Entity, With<ComboBox>>();

            query.single(world).unwrap()
        };

        let popup = world
            .query_filtered::<Entity, With<ComboBoxPopup>>()
            .single(world)
            .unwrap();
        *world.get_mut::<Visibility>(popup).unwrap() = Visibility::Visible;

        world.trigger(SetComboBoxSelected {
            entity: combo_box,
            selected: 2,
        });

        world.flush();

        assert_eq!(
            *world.get::<Visibility>(popup).unwrap(),
            Visibility::Visible
        );
        assert_eq!(world.resource::<ListBoxEvents>().0, 0);
        let received = world.resource::<ReceivedValue>();

        assert_eq!(received.source, None);
        assert_eq!(received.value, None);
    }

    // 将一个弹层与另一控件的选项组合成事件，验证两个控件及外部通知均不受影响。
    #[test]
    fn value_change_with_option_from_another_combo_box_should_be_ignored() {
        let mut app = App::new();

        app.init_resource::<ReceivedValue>()
            .add_plugins(ComboBoxPlugin)
            .add_observer(record_combo_box_value_change)
            .add_systems(Startup, spawn_two_test_combo_boxes);

        app.update();

        let world = app.world_mut();

        let (combo_a, combo_b) = {
            let combos = world.resource::<TestComboBoxes>();
            (combos.a, combos.b)
        };

        let (popup_a, popup_b) = {
            let mut query = world.query_filtered::<(Entity, &ChildOf), With<ComboBoxPopup>>();

            let mut popup_a = None;
            let mut popup_b = None;

            for (entity, parent) in query.iter(world) {
                if parent.parent() == combo_a {
                    popup_a = Some(entity);
                } else if parent.parent() == combo_b {
                    popup_b = Some(entity);
                }
            }

            (popup_a.unwrap(), popup_b.unwrap())
        };

        let option_b_2 = {
            let mut query = world.query::<(Entity, &ComboBoxOption, &ChildOf)>();

            query
                .iter(world)
                .find(|(_, option, parent)| parent.parent() == popup_b && option.index == 2)
                .map(|(entity, _, _)| entity)
                .unwrap()
        };

        // 让 A 的 Popup 处于打开状态，
        // 这样还能验证非法事件不会顺便把它关闭。
        *world.get_mut::<Visibility>(popup_a).unwrap() = Visibility::Visible;

        // 错误组合：
        // source 是 Popup A，
        // value 却来自 ComboBox B。
        world.trigger(ValueChange::<Entity> {
            source: popup_a,
            value: option_b_2,
            is_final: true,
        });

        world.flush();

        // A 的选择仍然保持 index = 1
        {
            let mut query = world.query::<(&ComboBoxOption, Has<Selected>, &ChildOf)>();

            for (option, selected, parent) in query.iter(world) {
                if parent.parent() == popup_a {
                    assert_eq!(selected, option.index == 1);
                }
            }
        }

        // B 的选择也不应该受到影响
        {
            let mut query = world.query::<(&ComboBoxOption, Has<Selected>, &ChildOf)>();

            for (option, selected, parent) in query.iter(world) {
                if parent.parent() == popup_b {
                    assert_eq!(selected, option.index == 1);
                }
            }
        }

        // A 的 Popup 也不能因为这个非法 ValueChange 被关闭
        assert_eq!(
            *world.get::<Visibility>(popup_a).unwrap(),
            Visibility::Visible
        );

        // 更不能对外宣称 A 的值变成了 2
        let received = world.resource::<ReceivedValue>();

        assert_eq!(received.source, None);
        assert_eq!(received.value, None);
    }
}
