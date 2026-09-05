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

/// Headless ComboBox 的根组件。
///
/// 挂载这个组件的 Entity 代表整个 ComboBox。
#[derive(Component, Debug, Default)]
pub struct ComboBox;

/// ComboBox 内部的 Field。
#[derive(Component, Debug, Default)]
struct ComboBoxField;

/// ComboBox 内部的 Popup。
#[derive(Component, Debug, Default)]
struct ComboBoxPopup;

/// ComboBox 内部的一个可选项。
#[derive(Component, Debug)]
struct ComboBoxOption {
    index: usize,
}

pub struct ComboBoxPlugin;

impl Plugin for ComboBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(handle_combo_box_field_activate)
            .add_observer(handle_combo_box_value_change)
            .add_observer(handle_combo_box_outside_click)
            .add_observer(handle_combo_box_disabled)
            .add_observer(handle_set_combo_box_selected);
    }
}

fn handle_combo_box_field_activate(
    event: On<Activate>,
    q_field: Query<&ChildOf, With<ComboBoxField>>,
    q_combo_box: Query<Has<InteractionDisabled>, With<ComboBox>>,
    mut q_popup: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
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

    for (popup_parent, mut visibility) in &mut q_popup {
        if popup_parent.parent() == combo_box {
            *visibility = if *visibility == Visibility::Hidden {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };

            break;
        }
    }
}

fn handle_combo_box_value_change(
    event: On<ValueChange<Entity>>,
    q_popup_parent: Query<&ChildOf, With<ComboBoxPopup>>,
    q_combo_box: Query<Has<InteractionDisabled>, With<ComboBox>>,
    q_children: Query<&Children>,
    q_option: Query<(&ComboBoxOption, &ChildOf)>,
    q_selected: Query<(Entity, Has<Selected>), With<ComboBoxOption>>,
    mut q_visibility: Query<&mut Visibility, With<ComboBoxPopup>>,
    mut commands: Commands,
) {
    // 只处理属于 ComboBox 的 ListBox
    let Ok(popup_parent) = q_popup_parent.get(event.source) else {
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

    // 更新 Selected
    if let Ok(children) = q_children.get(event.source) {
        for &child in children.iter() {
            let Ok((entity, selected)) = q_selected.get(child) else {
                continue;
            };

            if entity == event.value {
                if !selected {
                    commands.entity(entity).insert(Selected);
                }
            } else if selected {
                commands.entity(entity).remove::<Selected>();
            }
        }
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

pub fn spawn_headless_combo_box(
    commands: &mut Commands,
    option_count: usize,
    selected: Option<usize>,
) -> Entity {
    if let Some(selected) = selected {
        assert!(
            selected < option_count,
            "selected index must be within option_count"
        );
    }

    let mut combo_box = commands.spawn(ComboBox);

    combo_box.with_children(|root| {
        root.spawn((ComboBoxField, Button));

        root.spawn((ComboBoxPopup, ListBox, Visibility::Hidden))
            .with_children(|popup| {
                for index in 0..option_count {
                    let mut option = popup.spawn((ComboBoxOption { index }, ListItem));

                    if Some(index) == selected {
                        option.insert(Selected);
                    }
                }
            });
    });

    let combo_box_entity = combo_box.id();

    combo_box_entity
}

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

fn handle_combo_box_disabled(
    event: On<Add, InteractionDisabled>,
    q_combo_box: Query<&Children, With<ComboBox>>,
    mut q_popup: Query<&mut Visibility, With<ComboBoxPopup>>,
) {
    let Ok(children) = q_combo_box.get(event.entity) else {
        return;
    };

    for &child in children.iter() {
        let Ok(mut visibility) = q_popup.get_mut(child) else {
            continue;
        };

        *visibility = Visibility::Hidden;
        break;
    }
}

#[derive(EntityEvent, Debug)]
pub struct SetComboBoxSelected {
    #[event_target]
    pub entity: Entity,
    pub selected: Option<usize>,
}

fn handle_set_combo_box_selected(
    event: On<SetComboBoxSelected>,
    q_combo_box: Query<&Children, With<ComboBox>>,
    q_children: Query<&Children>,
    q_options: Query<(Entity, &ComboBoxOption, Has<Selected>)>,
    mut commands: Commands,
) {
    let Ok(combo_box_children) = q_combo_box.get(event.entity) else {
        return;
    };

    let mut options = Vec::new();

    for &child in combo_box_children.iter() {
        let Ok(popup_children) = q_children.get(child) else {
            continue;
        };

        for &option_entity in popup_children.iter() {
            let Ok((entity, option, selected)) = q_options.get(option_entity) else {
                continue;
            };

            options.push((entity, option.index, selected));
        }
    }

    // Some(index) 必须确实存在
    if let Some(index) = event.selected
        && !options
            .iter()
            .any(|(_, option_index, _)| *option_index == index)
    {
        return;
    }

    for (entity, index, selected) in options {
        let should_select = event.selected == Some(index);

        if should_select && !selected {
            commands.entity(entity).insert(Selected);
        } else if !should_select && selected {
            commands.entity(entity).remove::<Selected>();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    use bevy::{
        app::{App, Startup},
        camera::NormalizedRenderTarget,
        ecs::{
            entity::Entity, hierarchy::ChildOf, query::With, resource::Resource, system::ResMut,
        },
        math::Vec2,
        picking::{
            backend::HitData,
            pointer::{Location, PointerButton, PointerId},
        },
        ui::Selected,
    };

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
        spawn_headless_combo_box(&mut commands, 3, Some(1));
    }

    fn primary_click(entity: Entity) -> Pointer<Click> {
        Pointer::new(
            PointerId::Mouse,
            Location {
                target: NormalizedRenderTarget::None {
                    width: 1,
                    height: 1,
                },
                position: Vec2::ZERO,
            },
            Click {
                button: PointerButton::Primary,
                hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
                duration: Duration::ZERO,
                count: 1,
            },
            entity,
        )
    }

    #[derive(Resource)]
    struct TestComboBoxes {
        a: Entity,
        b: Entity,
    }

    fn spawn_two_test_combo_boxes(mut commands: Commands) {
        let a = spawn_headless_combo_box(&mut commands, 3, Some(1));
        let b = spawn_headless_combo_box(&mut commands, 3, Some(1));

        commands.insert_resource(TestComboBoxes { a, b });
    }

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
            selected: Some(2),
        });

        world.flush();

        let mut query = world.query::<(&ComboBoxOption, Has<Selected>)>();

        for (option, selected) in query.iter(world) {
            assert_eq!(selected, option.index == 2);
        }
    }

    #[test]
    fn programmatic_selection_none_should_clear_selection() {
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
            selected: None,
        });

        world.flush();

        let mut query = world.query::<(&ComboBoxOption, Has<Selected>)>();

        for (_, selected) in query.iter(world) {
            assert!(!selected);
        }
    }

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
            selected: Some(999),
        });

        world.flush();

        let mut query = world.query::<(&ComboBoxOption, Has<Selected>)>();

        for (option, selected) in query.iter(world) {
            assert_eq!(selected, option.index == 1);
        }
    }

    #[test]
    fn programmatic_selection_should_not_emit_value_change() {
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

        world.trigger(SetComboBoxSelected {
            entity: combo_box,
            selected: Some(2),
        });

        world.flush();

        let received = world.resource::<ReceivedValue>();

        assert_eq!(received.source, None);
        assert_eq!(received.value, None);
    }

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
