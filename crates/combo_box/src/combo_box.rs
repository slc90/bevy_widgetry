use crate::{
    field,
    popup::{self, ComboBoxPopup},
};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_log::widgetry_error;

/// 直接消费独立 ListModel 的不可编辑 ComboBox，需通过 WidgetryComboBoxAppExt 注册 T。
/// source 与 renderer 必填且创建后固定；source 必须持续具有匹配的 ListModel。
/// selection 使用 source-local stable id，用户通知为 root `ValueChange<WidgetryListItemId>`。
/// 非空 model 仅在初始化时默认选择第一项；已有 selection 优先，删除后不自动改选。
/// 内部 WidgetryListViewState.selected 是唯一 authority；Field 根据 item id/index/revision 派生内容。
/// 空 model 初始化后 push 不自动选择；无 selection 时保留 Button 与箭头，多个 view 可独立选择。
/// Popup 高度为最大可见行数范围内的内容高度加 border；打开后内部 ListView 接管 focus 和 navigation。
/// 用户改值或有效重选关闭 Popup，Escape 返回 Field focus；空 model 不打开并关闭已展开 Popup。
/// 关闭后释放滞留的内部 ListView focus，outside click 不覆盖被点击目标的 focus。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryComboBoxProps<T>)]
pub struct WidgetryComboBox<T: Send + Sync + 'static> {
    /// 所有 option 业务数据与 identity 的唯一来源。
    source: Entity,
    /// 每行的 logical px 高度，创建后固定。
    item_height: f32,
    /// Popup viewport 的最大行数，创建后固定。
    max_visible_items: usize,
    /// 业务内容 factory，直接复用 ListView renderer。
    renderer: WidgetryListViewRenderer<T>,
}

/// 一次性 Scene 输入，展开后不保留 props state；source 与 renderer 必填。
pub struct WidgetryComboBoxProps<T: Send + Sync + 'static> {
    /// 生命周期内持有匹配 WidgetryListModel<T> 的独立 entity。
    pub source: Entity,
    /// 有限正数，默认 32 logical px。
    pub item_height: f32,
    /// 非零的最大可见行数，默认 8。
    pub max_visible_items: usize,
    /// 可重复调用的业务内容 renderer，不要求 T 实现 Clone。
    pub renderer: WidgetryListViewRenderer<T>,
}

/// 默认 selection 已完成一次性初始化，空 model 也必须标记，避免后续 push 自动选择。
#[derive(Component)]
pub(crate) struct Initialized;

/// 只初始化当前 root 的内部 ListView，不为 ComboBox 建立 selection 副本。
pub(crate) fn initialize_selection<T: Send + Sync + 'static>(
    roots: Query<(Entity, &WidgetryComboBox<T>, &Children), Without<Initialized>>,
    popups: Query<&Children, With<ComboBoxPopup>>,
    lists: Query<&WidgetryListViewState, With<WidgetryListView<T>>>,
    models: Query<&WidgetryListModel<T>>,
    mut commands: Commands,
) {
    for (root, combo, children) in &roots {
        let list = children.iter().find_map(|popup| {
            popups
                .get(popup)
                .ok()?
                .iter()
                .find(|&child| lists.contains(child))
        });
        let Some(list) = list else {
            widgetry_error!(?root, "ComboBox 缺少内部 ListView");
            continue;
        };
        let Ok(model) = models.get(combo.source) else {
            // ListView 的 source validation 负责报告这一公开前置条件错误。
            continue;
        };
        if lists.get(list).is_ok_and(|state| state.selected.is_none()) && !model.is_empty() {
            WidgetryListView::<T>::set_selected(&mut commands, list, 0);
        }
        commands.entity(root).insert(Initialized);
    }
}

/// 用户改值后关闭 Popup，并将 stable id 通知重新定位到 root；Field 仍从真实 state 派生。
pub(crate) fn handle_value_change<T: Send + Sync + 'static>(
    event: On<ValueChange<WidgetryListItemId>>,
    lists: Query<&ChildOf, With<WidgetryListView<T>>>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
    roots: Query<(), (With<WidgetryComboBox<T>>, Without<InteractionDisabled>)>,
    focus: Option<ResMut<InputFocus>>,
    mut commands: Commands,
) {
    let Ok(list_parent) = lists.get(event.source) else {
        return;
    };
    let Ok((popup_parent, mut visibility)) = popups.get_mut(list_parent.parent()) else {
        return;
    };
    let root = popup_parent.parent();
    if roots.contains(root) {
        *visibility = Visibility::Hidden;
        // 同帧 input 已派发到旧目标；立即释放 focus，让 ListView 拒绝剩余的 queued keyboard 操作。
        if let Some(mut focus) = focus
            && focus.get() == Some(event.source)
        {
            focus.clear();
        }
        commands.trigger(ValueChange {
            source: root,
            value: event.value,
            is_final: event.is_final,
        });
    }
}

impl<T: Send + Sync + 'static> WidgetryComboBox<T> {
    /// 读取创建后固定的 source，stable id 必须在这个 model 上解释。
    pub fn source(&self) -> Entity {
        self.source
    }

    /// 读取固定的 row 高度。
    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    /// 读取固定的 Popup viewport 行数上限。
    pub fn max_visible_items(&self) -> usize {
        self.max_visible_items
    }

    /// 读取与内部 ListView 共用的 renderer factory。
    pub fn renderer(&self) -> &WidgetryListViewRenderer<T> {
        &self.renderer
    }

    /// 按执行时 source 的 stable id 静默设置内部 ListView selection。
    /// 无效 root 或当前 model 中不存在的 id 为 no-op；root/item disabled 不阻止程序化设置。
    /// 不发送用户通知、不关闭 Popup；Field 在下一次 PostUpdate 根据最后真实 selection 收敛。
    pub fn set_selected(commands: &mut Commands, entity: Entity, item_id: WidgetryListItemId) {
        commands.queue(move |world: &mut World| {
            let Some(combo) = world.get::<Self>(entity) else {
                return;
            };
            let Some(index) = world
                .get::<WidgetryListModel<T>>(combo.source)
                .and_then(|model| model.index_of(item_id))
            else {
                return;
            };
            let list = world.get::<Children>(entity).and_then(|children| {
                children.iter().find_map(|popup| {
                    world.get::<ComboBoxPopup>(popup)?;
                    world
                        .get::<Children>(popup)?
                        .iter()
                        .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())
                })
            });
            let Some(list) = list else {
                widgetry_error!(?entity, "ComboBox 缺少内部 ListView");
                return;
            };
            WidgetryListView::<T>::set_selected(&mut world.commands(), list, index);
        });
    }

    /// 构造 Button 与 ListView 组合；ListView 检查必填 source、renderer 和 row 高度。
    fn scene(props: WidgetryComboBoxProps<T>) -> impl Scene {
        let height = props.item_height * props.max_visible_items as f32 + 2.0;
        if props.max_visible_items == 0 || !height.is_finite() {
            widgetry_error!(source = ?props.source, max_visible_items = props.max_visible_items, "ComboBox viewport 高度必须有限且行数非零");
            panic!("WidgetryComboBox requires a finite viewport and nonzero max_visible_items");
        }
        let popup = popup::scene::<T>(props.source, props.item_height, props.renderer.clone());
        bsn! {
            WidgetryComboBox::<T> {
                source: {props.source}, item_height: {props.item_height},
                max_visible_items: {props.max_visible_items}, renderer: {props.renderer},
            }
            Node { width: px(200) }
            Children [field::scene(), ({popup})]
        }
    }
}

impl<T: Send + Sync + 'static> Default for WidgetryComboBoxProps<T> {
    fn default() -> Self {
        Self {
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            max_visible_items: 8,
            renderer: WidgetryListViewRenderer::default(),
        }
    }
}
