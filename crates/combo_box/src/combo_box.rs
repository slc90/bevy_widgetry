use crate::{
    field,
    popup::{self, ComboBoxPopup},
};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::collections::VecDeque;

/// 直接消费独立 ListModel 的不可编辑 ComboBox，需通过 WidgetryComboBoxAppExt 注册 T。
/// source 与 renderer 必填且创建后固定；source 必须持续具有匹配的 ListModel。
/// selection 使用 source-local stable id，UI / 程序实际变化提交后发 root ValueChange<Option<WidgetryListItemId>>。
/// Some 为选中 id，None 为显式清空；同值不通知，无 origin，内部 ListView 通知也可能被 App observer 收到。
/// 非空 model 仅在初始化时默认选择第一项；已有 selection 优先，删除后不自动改选。
/// 内部 WidgetryListViewState.selected 是唯一 authority；Field 根据 item id/index/revision 派生内容。
/// 空 model 初始化后 push 不自动选择；无 selection 时保留 Button 与箭头，多个 view 可独立选择。
/// Popup 高度为最大可见行数范围内的内容高度加 border；打开后内部 ListView 接管 focus 和 navigation。
/// 用户改值或有效重选关闭 Popup，Escape 返回 Field focus；空 model 不打开并关闭已展开 Popup。
/// 关闭后释放滞留的内部 ListView focus，outside click 不覆盖被点击目标的 focus。
/// 程序 set_selected / clear_selection 保留 Popup 与 focus，disabled 不阻止合法程序通知；初始化 / repair 静默。
/// root observer 可读内部 selection，但 Field renderer / layout 仍在后续阶段派生。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryComboBoxProps<T>)]
#[require(ComboDiagnostics, ForwardedChanges)]
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

/// 固定 root 拥有每个行为的异常边界，不持有额外 selection authority。
#[derive(Component, Default)]
pub(crate) struct ComboDiagnostics {
    pub(crate) projection: FailureState,
    pub(crate) icon: FailureState,
    initialization: FailureState,
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

/// fallible template 的私有输出，证明一次性配置校验完成，避免与调用方 Node patch 冲突。
#[derive(Component)]
struct ValidatedConfig;

/// 默认 selection 已完成一次性初始化，空 model 也必须标记，避免后续 push 自动选择。
#[derive(Component)]
pub(crate) struct Initialized;

/// 程序请求委托 ListView 时保留 Popup；只用于内部路由，不是公开 origin 或 selection authority。
#[derive(Component)]
pub(crate) struct PreservePopup;

/// 尚未派发的 root event payload，不参与 selection 查询或 Field projection。
#[derive(Component, Default)]
pub(crate) struct ForwardedChanges(VecDeque<ValueChange<Option<WidgetryListItemId>>>);

/// 只初始化当前 root 的内部 ListView，不为 ComboBox 建立 selection 副本。
pub(crate) fn initialize_selection<T: Send + Sync + 'static>(
    mut roots: Query<
        (
            Entity,
            &WidgetryComboBox<T>,
            &Children,
            &mut ComboDiagnostics,
        ),
        Without<Initialized>,
    >,
    popups: Query<&Children, With<ComboBoxPopup>>,
    lists: Query<&WidgetryListViewState, With<WidgetryListView<T>>>,
    models: Query<&WidgetryListModel<T>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for (root, combo, children, mut diagnostics) in &mut roots {
        let result = (|| -> Result<(), BevyError> {
            let list = children.iter().find_map(|popup| {
                popups
                    .get(popup)
                    .ok()?
                    .iter()
                    .find(|&child| lists.contains(child))
            });
            let Some(list) = list else {
                return Err(BevyError::error("ComboBox internal ListView missing"));
            };
            let Ok(model) = models.get(combo.source) else {
                // ListView 的 source validation 负责报告这一公开前置条件错误。
                return Ok(());
            };
            if lists.get(list).is_ok_and(|state| state.selected.is_none()) && !model.is_empty() {
                project_selection::<T>(&mut commands, list, 0);
            }
            commands.entity(root).insert(Initialized);

            Ok(())
        })();
        let result = diagnostics.initialization.observe(
            result,
            |error| widgetry_error!(?root, %error, "ComboBox selection 初始化失败"),
            || widgetry_info!(?root, "ComboBox selection 初始化恢复正常"),
        );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

/// 已提交的 ListView 通知重新定位到 root；仅用户改选附带关闭，程序委托期间保留 Popup / focus。
pub(crate) fn handle_value_change<T: Send + Sync + 'static>(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    lists: Query<(&ChildOf, Has<PreservePopup>), With<WidgetryListView<T>>>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
    mut roots: Query<(Has<InteractionDisabled>, &mut ForwardedChanges), With<WidgetryComboBox<T>>>,
    focus: Option<ResMut<InputFocus>>,
    mut commands: Commands,
) {
    let Ok((list_parent, preserve)) = lists.get(event.source) else {
        return;
    };
    let Ok((popup_parent, mut visibility)) = popups.get_mut(list_parent.parent()) else {
        return;
    };
    let root = popup_parent.parent();
    let Ok((disabled, mut changes)) = roots.get_mut(root) else {
        return;
    };
    if !disabled && !preserve && event.value.is_some() {
        *visibility = Visibility::Hidden;
        // 同帧 input 已派发到旧目标；立即释放 focus，让 ListView 拒绝剩余的 queued keyboard 操作。
        if let Some(mut focus) = focus
            && focus.get() == Some(event.source)
        {
            focus.clear();
        }
    }
    changes.0.push_back(ValueChange {
        source: root,
        value: event.value,
        is_final: event.is_final,
    });
    commands.queue(move |world: &mut World| dispatch_value_changes(world, root));
}

/// 后续程序请求可能由内部 event 的其他 observer 先入队；提交前先派发已有通知，避免 payload 倒序。
fn dispatch_value_changes(world: &mut World, root: Entity) {
    while let Some(event) = world
        .get_mut::<ForwardedChanges>(root)
        .and_then(|mut changes| changes.0.pop_front())
    {
        world.trigger(event);
    }
}

/// 独立 command 派发旧通知，使其 observer 的 deferred CRUD 先执行，再解析本次请求的 stable id。
fn queue_pending_changes<T: Send + Sync + 'static>(commands: &mut Commands, root: Entity) {
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryComboBox<T>>(root).is_some() {
            dispatch_value_changes(world, root);
        }
    });
}

/// 一次性初始化维护原有静默 projection，避免借用公开 selection 通知。
fn project_selection<T: Send + Sync + 'static>(
    commands: &mut Commands,
    list: Entity,
    index: usize,
) {
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let id = world
            .get::<WidgetryListView<T>>(list)
            .and_then(|view| world.get::<WidgetryListModel<T>>(view.source()))
            .and_then(|model| model.id(index));
        let Some(id) = id else {
            widgetry_error!(?list, index, "ComboBox 内部 selection projection 目标失效");
            return Err(BevyError::error(
                "ComboBox internal selection projection target missing",
            ));
        };
        if world.get::<WidgetryListViewState>(list).is_none() {
            widgetry_error!(?list, "ComboBox 内部 selection state 缺失");
            return Err(BevyError::error(
                "ComboBox internal selection state missing",
            ));
        }
        world.entity_mut(list).insert(WidgetryListViewState {
            selected: Some(id),
            active: Some(id),
        });
        WidgetryListView::<T>::set_active(&mut world.commands(), list, Some(index));
        Ok(())
    });
}

/// 无效程序请求在写入前拒绝，日志与宿主 Error 区分合法同值。
fn update_error(root: Entity, reason: &str) -> BevyError {
    widgetry_error!(?root, reason, "ComboBox state 更新失败");
    BevyError::error(format!("ComboBox state 更新失败: {reason}"))
}

/// 校验 typed root、source、内部 shell 与 authority；不改变初始化或 Model repair。
fn update_list<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
) -> Result<(Entity, Entity), BevyError> {
    let combo = world
        .get::<WidgetryComboBox<T>>(root)
        .ok_or_else(|| update_error(root, "Widget 不存在或 type 不匹配"))?;
    let source = combo.source();
    if world.get::<WidgetryListModel<T>>(source).is_none() {
        return Err(update_error(root, "source 不存在或 Model type 不匹配"));
    }
    let list = world
        .get::<Children>(root)
        .and_then(|children| {
            children.iter().find_map(|popup| {
                world.get::<ComboBoxPopup>(popup)?;
                world.get::<Visibility>(popup)?;
                world
                    .get::<Children>(popup)?
                    .iter()
                    .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())
            })
        })
        .ok_or_else(|| update_error(root, "内部 ListView 缺失"))?;
    if world
        .get::<WidgetryListView<T>>(list)
        .is_none_or(|view| view.source() != source)
        || world.get::<WidgetryListViewState>(list).is_none()
    {
        return Err(update_error(root, "内部 ListView source 或 state 无效"));
    }
    Ok((list, source))
}

/// nested commands 在下一条外部请求前执行完；通知转发不混入程序更新的 Popup 关闭副作用。
fn queue_selection<T: Send + Sync + 'static>(
    world: &mut World,
    list: Entity,
    index: Option<usize>,
) {
    let already_preserved = world.get::<PreservePopup>(list).is_some();
    world.entity_mut(list).insert(PreservePopup);
    match index {
        Some(index) => WidgetryListView::<T>::set_selected(&mut world.commands(), list, index),
        None => WidgetryListView::<T>::clear_selection(&mut world.commands(), list),
    }
    if !already_preserved {
        world.commands().queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(list) {
                entity.remove::<PreservePopup>();
            }
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

    /// Commands 执行时按所属 source 解析 stable id，委托 ListView 更新 selected / active 与 reveal。
    /// 实际改选在 authority 提交后发 root ValueChange<Option<WidgetryListItemId>>；同值不通知。
    /// root/item disabled 不阻止设置与通知，不关闭 Popup 或改变 focus，Field 后续派生。
    /// 无效 typed root、source、shell、state 或 stale id 记录 ERROR 并以 Severity::Error 交给宿主，失败不写 state。
    pub fn set_selected(commands: &mut Commands, entity: Entity, item_id: WidgetryListItemId) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, source) = update_list::<T>(world, entity)?;
            let index = world
                .get::<WidgetryListModel<T>>(source)
                .and_then(|model| model.index_of(item_id))
                .ok_or_else(|| update_error(entity, "id 不存在于当前 source"))?;
            queue_selection::<T>(world, list, Some(index));
            Ok(())
        });
    }

    /// 显式清空 selection，保留 active、focus、Popup 与 scroll / reveal；已空不通知。
    /// 非空到 None 提交后发 root ValueChange<Option<WidgetryListItemId>>，错误语义同 set_selected。
    pub fn clear_selection(commands: &mut Commands, entity: Entity) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, _) = update_list::<T>(world, entity)?;
            queue_selection::<T>(world, list, None);
            Ok(())
        });
    }

    /// 构造 Button 与 ListView 组合；ListView 检查必填 source、renderer 和 row 高度。
    fn scene(props: WidgetryComboBoxProps<T>) -> impl Scene {
        let height = props.item_height * props.max_visible_items as f32 + 2.0;
        let max_visible_items = props.max_visible_items;
        let source = props.source;
        let popup = popup::scene::<T>(props.source, props.item_height, props.renderer.clone());
        bsn! {
            WidgetryComboBox::<T> {
                source: {props.source}, item_height: {props.item_height},
                max_visible_items: {props.max_visible_items}, renderer: {props.renderer},
            }
            template(move |_| {
                if max_visible_items == 0 || !height.is_finite() {
                    widgetry_error!(source = ?source, max_visible_items = max_visible_items, "ComboBox viewport 高度必须有限且行数非零");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryComboBox requires a finite viewport and nonzero max_visible_items"));
                }
                Ok(ValidatedConfig)
            })
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
