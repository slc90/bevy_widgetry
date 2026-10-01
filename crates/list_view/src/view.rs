use crate::behavior::ListNavigation;
use crate::{WidgetryListItemId, WidgetryListModel};
use bevy::a11y::AccessibilityNode;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui_widgets::ActiveDescendant;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use bevy_widgetry_scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
};
use std::sync::Arc;

/// 通过 BSN 的 @WidgetryListView::<T> 构造的长期 ECS identity。
/// 需注册 WidgetryListViewPlugin，并通过 WidgetryListViewAppExt 注册 T。
/// source 必须始终持有匹配的 WidgetryListModel<T>；构造配置固定，不能替换 component。
/// 必填 prop 与高度在 Scene 构造时检查，source 存在性和 type 在每次 PreUpdate 检查；错误记录 ERROR 并上抛 BevyError。
/// 同 root 组合纵向 ScrollArea，隐藏 scrollbar；wheel、trackpad 与原生 ScrollPosition 保持可用。
/// 调用方须通过 root Node patch 或父 flex/grid 提供有界纵向 layout，并在 ancestor UI root 配置 TabGroup。
/// runtime 仅实例化真实 viewport 内的 rows，按 entry id/revision 管理 renderer lifecycle。
/// selection/active 以 source-local stable id 为 authority，row 仅投影 Selected 与 ActiveDescendant。
/// root/item disabled 限制用户输入，不阻止 set_selected、直接 ScrollPosition 更新或 model CRUD。
/// 默认外框为 4px 圆角；内部内容使用 Bevy 原生矩形 overflow clip，不沿外框圆角裁剪。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryListViewProps<T>)]
#[require(WidgetryListViewState, ListNavigation, ListDiagnostics)]
pub struct WidgetryListView<T: Send + Sync + 'static> {
    /// 所有业务内容和 item identity 的唯一来源。
    source: Entity,
    /// 创建后固定的 row 高度，单位为 logical px。
    item_height: f32,
    /// 创建后固定的业务内容 factory。
    renderer: WidgetryListViewRenderer<T>,
}

/// 固定 root 所拥有的独立 source/runtime 诊断；不依赖 system 执行次数。
#[derive(Component, Default)]
pub(crate) struct ListDiagnostics {
    source: FailureState,
    pub(crate) runtime: FailureState,
}

/// 只用于 Scene 构造；source 与 renderer 必填，item_height 默认 32 logical px。
pub struct WidgetryListViewProps<T: Send + Sync + 'static> {
    /// 生命周期内必须存在且具有匹配 model component 的 entity。
    pub source: Entity,
    /// 必须为有限正数，创建后固定。
    pub item_height: f32,
    /// 构造 row direct children 的 factory，创建后固定。
    pub renderer: WidgetryListViewRenderer<T>,
}

/// 将 owned/'static SceneList factory type erase，不要求业务 T 实现 Clone。
/// factory 只生成 row wrapper 的 direct children，不负责 ListItem、row identity、
/// height/padding/border、交互 state、disabled 或 foreground/theme。
/// subtree 在滚出 viewport 或 revision 变化时可能销毁；持久业务 state 应放在 model 或其他 ECS state。
/// Default 仅用于 BSN template 的未配置占位；构造 view 或 render 前必须通过 new 提供 factory。
pub struct WidgetryListViewRenderer<T>(
    Option<Arc<dyn Fn(usize, &T) -> Box<dyn SceneList> + Send + Sync>>,
);

/// offscreen item 同样保有的业务权威 state；物理 row 上的 state 只是 hierarchy projection。
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WidgetryListViewState {
    /// logical selection 的稳定 id；None 表示未选中。
    pub selected: Option<WidgetryListItemId>,
    /// logical active 的稳定 id；None 表示无 active item。
    pub active: Option<WidgetryListItemId>,
}

/// rendered row 的公开 identity，也是 descendant click 向上解析的边界。
/// index 是当前 model 顺序的 projection，业务 identity 应使用 id。
/// item disabled 保留 focused active border，仍抑制 hover/pressed/selected background 并使用 disabled foreground。
/// root disabled 或 root 失去 focus 时，不显示 row active border。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryListViewItem {
    /// 该 row 对应的 model-local identity。
    pub id: WidgetryListItemId,
    /// 该 row 当前对应的 model index。
    pub index: usize,
}

/// 占据当前 rendered range 之前的纵向空间，初始化为空，不承担 focus 或 picking。
#[derive(Component, Default, Clone)]
pub(crate) struct TopSpacer;

/// 占据当前 rendered range 之后的纵向空间，初始化为空，不承担 focus 或 picking。
#[derive(Component, Default, Clone)]
pub(crate) struct BottomSpacer;

/// typed runtime 检查 source invariant；后续 row reconciliation 继续使用同一 source contract。
pub(crate) fn validate_sources<T: Send + Sync + 'static>(
    world: &mut World,
) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = validate_source::<T>(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

/// PreUpdate 与 PostUpdate 都检查真实 source，但仅 root 的异常状态决定是否记录。
pub(crate) fn validate_source<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let source = world
        .get::<WidgetryListView<T>>(root)
        .map(WidgetryListView::source)
        .unwrap_or(Entity::PLACEHOLDER);
    let result = if world.get::<WidgetryListModel<T>>(source).is_some() {
        Ok(())
    } else {
        Err(BevyError::error(
            "WidgetryListView requires a live matching WidgetryListModel source",
        ))
    };
    let Some(mut diagnostics) = world.get_mut::<ListDiagnostics>(root) else {
        return result;
    };
    diagnostics.source.observe(result,
        |error| widgetry_error!(entity = ?root, ?source, item_type = std::any::type_name::<T>(), %error, "ListView source 不存在或缺少匹配的 ListModel"),
        || widgetry_info!(entity = ?root, ?source, "ListView source 恢复正常"))
}

impl<T: Send + Sync + 'static> WidgetryListView<T> {
    /// 按当前 index 静默设置 logical selection/active 并确保目标可见。
    /// invalid list/index 为 no-op；root/item disabled 不阻止 programmatic 设置。
    pub fn set_selected(commands: &mut Commands, list: Entity, index: usize) {
        commands.queue(move |world: &mut World| {
            crate::behavior::set_selected::<T>(world, list, index);
        });
    }

    /// 读取创建后固定的 source entity。
    pub fn source(&self) -> Entity {
        self.source
    }

    /// 读取创建后固定的 logical row 高度。
    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    /// 读取创建后固定的业务内容 factory。
    pub fn renderer(&self) -> &WidgetryListViewRenderer<T> {
        &self.renderer
    }

    /// 拒绝不可恢复的配置错误，再将 props 一次性写入持久 component。
    fn scene(props: WidgetryListViewProps<T>) -> impl Scene {
        let source = props.source;
        let item_height = props.item_height;
        let renderer_missing = props.renderer.0.is_none();
        bsn! {
            template(move |_| {
                if source == Entity::PLACEHOLDER || renderer_missing {
                    widgetry_error!(source = ?source, "ListView 构造必须提供 source 和 renderer");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryListView requires source and renderer"));
                }
                if !item_height.is_finite() || item_height <= 0.0 {
                    widgetry_error!(source = ?source, item_height = item_height, "ListView item_height 必须为有限正数");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryListView requires a finite positive item_height"));
                }
                Ok(ListNavigation::default())
            })
            @WidgetryScrollArea {
                @axis: ScrollAxis::Vertical,
                @scrollbar_visibility: {ScrollbarVisibility { horizontal: ScrollbarPolicy::Hidden, vertical: ScrollbarPolicy::Hidden }},
                @keyboard_scroll: false,
                @children: bsn_list![
                    (TopSpacer Node { height: px(0), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))),
                    (BottomSpacer Node { height: px(0), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))),
                ],
            }
            TabIndex::default()
            ActiveDescendant::default()
            template(|_| Ok(AccessibilityNode(accesskit::Node::new(accesskit::Role::ListBox))))
            BorderColor::default()
            Node {
                min_width: px(0), min_height: px(0), border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)), overflow: Overflow::clip(),
            }
            WidgetryListView::<T> {
                source: {props.source},
                item_height: {props.item_height},
                renderer: {props.renderer},
            }
        }
    }
}

impl<T: Send + Sync + 'static> Default for WidgetryListViewProps<T> {
    fn default() -> Self {
        Self {
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            renderer: WidgetryListViewRenderer(None),
        }
    }
}

impl<T> Clone for WidgetryListViewRenderer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for WidgetryListViewRenderer<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> WidgetryListViewRenderer<T> {
    /// 接收可重复调用的 factory；返回的 SceneList 不得长期借用传入的 value。
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn(usize, &T) -> S + Send + Sync + 'static,
    {
        Self(Some(Arc::new(move |index, value| {
            Box::new(factory(index, value))
        })))
    }

    /// 构造一次独立业务内容，不包含 row wrapper。
    pub fn render(&self, index: usize, value: &T) -> Result<Box<dyn SceneList>, BevyError> {
        let Some(factory) = &self.0 else {
            widgetry_error!(index, "ListView renderer 缺失");
            return Err(BevyError::error("WidgetryListView requires renderer"));
        };
        Ok(factory(index, value))
    }
}
