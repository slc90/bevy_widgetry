use crate::{field, option::ComboBoxOption, popup::ComboBoxPopup};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_log::widgetry_error;
use std::sync::Arc;

/// 非编辑式选择控件；通过 BSN 的 `@WidgetryComboBox` 构造，需注册 WidgetryComboBoxPlugin。
/// 初始选择索引 0；用户实际改值才发送root ValueChange<usize>，禁用状态只需挂在root上。
/// 选项固定不变，Field 是 Selected 的派生副本，切换时不保留副本内部状态。
#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryComboBoxProps)]
pub struct WidgetryComboBox;

/// BSN 的一次性选项输入；Default 仅满足 SceneComponent，实际构造时空列表会断言失败。
#[derive(Default)]
pub struct WidgetryComboBoxProps {
    /// 顺序即公开的选择索引；每项内容由可重复调用的 factory 构造。
    pub options: Vec<WidgetryComboBoxOptionFactory>,
}

/// 可重复构造选项 SceneList，供 Popup 与 Field 独立使用。
/// closure 捕获的拥有所有权数据须自行 clone，不能消费仅可使用一次的内容。
#[derive(Clone)]
pub struct WidgetryComboBoxOptionFactory(Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>);

/// root上的固定选项来源，Scene 展开后由它负责运行期 Field 重建。
#[derive(Component)]
pub(crate) struct ComboBoxOptions(pub(crate) Vec<WidgetryComboBoxOptionFactory>);

/// 仅处理本控件的列表通知，按命令顺序验证选择并提交真实用户改值。
pub(crate) fn handle_value_change(event: On<ValueChange<Entity>>, mut commands: Commands) {
    let (popup, target, is_final) = (event.source, event.value, event.is_final);
    commands.queue(move |world: &mut World| {
        if world.get::<ComboBoxPopup>(popup).is_none() {
            return;
        }
        let Some(root) = world.get::<ChildOf>(popup).map(ChildOf::parent) else {
            widgetry_error!(?popup, "ComboBox Popup缺少所属root");
            return;
        };
        if world.get::<WidgetryComboBox>(root).is_none()
            || world.get::<InteractionDisabled>(root).is_some()
            || world.get::<ChildOf>(target).map(ChildOf::parent) != Some(popup)
        {
            return;
        }
        let Some(index) = world
            .get::<ComboBoxOption>(target)
            .map(|option| option.index)
        else {
            return;
        };
        if select_option(world, root, index) {
            if let Some(mut visibility) = world.get_mut::<Visibility>(popup) {
                *visibility = Visibility::Hidden;
            }
            world.trigger(ValueChange {
                source: root,
                value: index,
                is_final,
            });
        }
    });
}

/// 验证完整索引后维持唯一 Selected；相同目标不写组件，以免触发内容重建。
fn select_option(world: &mut World, root: Entity, index: usize) -> bool {
    if world.get::<WidgetryComboBox>(root).is_none() {
        return false;
    }
    let Some(options) = world.get::<ComboBoxOptions>(root) else {
        widgetry_error!(?root, "ComboBox root缺少选项来源");
        return false;
    };
    if index >= options.0.len() {
        return false;
    }
    let popup = world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|&entity| world.get::<ComboBoxPopup>(entity).is_some())
    });
    let Some(rows) = popup
        .and_then(|popup| world.get::<Children>(popup))
        .map(|rows| rows.to_vec())
    else {
        widgetry_error!(?root, "ComboBox 缺少Popup选项层级");
        return false;
    };
    let Some(target) = rows.iter().copied().find(|&row| {
        world
            .get::<ComboBoxOption>(row)
            .is_some_and(|option| option.index == index)
    }) else {
        widgetry_error!(?root, index, "ComboBox 缺少对应索引的选项");
        return false;
    };
    if world.get::<Selected>(target).is_some() {
        return false;
    }
    for row in rows {
        if world.get::<ComboBoxOption>(row).is_some() && world.get::<Selected>(row).is_some() {
            world.entity_mut(row).remove::<Selected>();
        }
    }
    world.entity_mut(target).insert(Selected);
    true
}

impl WidgetryComboBox {
    /// 首次展开完整层级；默认首项内容与列表行分别构造。
    fn scene(props: WidgetryComboBoxProps) -> impl Scene {
        assert!(
            !props.options.is_empty(),
            "WidgetryComboBox requires at least one option"
        );
        let field = field::scene(props.options[0].build());
        let popup = crate::popup::scene(&props.options);
        bsn! {
            template(move |_| Ok(ComboBoxOptions(props.options.clone())))
            Node { width: px(200) }
            Children [{bsn_list![field, popup]}]
        }
    }

    /// 排队设置选择；无效root、越界或相同索引均无操作，禁用root仍允许设置。
    /// 不发送用户 ValueChange，也不改变 Popup 显隐；Field 在后续 Update 从 Selected 同步。
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: usize) {
        commands.queue(move |world: &mut World| {
            select_option(world, entity, selected);
        });
    }
}

impl WidgetryComboBoxOptionFactory {
    /// 接收可重复调用的 SceneList 工厂；每次调用产生独立实体内容。
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn() -> S + Send + Sync + 'static,
    {
        Self(Arc::new(move || Box::new(factory())))
    }

    /// 为某个展示位置构造一份可独立消费的内容。
    pub(crate) fn build(&self) -> Box<dyn SceneList> {
        (self.0)()
    }
}
