use crate::combo_box::WidgetryComboBox;
use crate::popup::ComboBoxPopup;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewState,
};
use bevy_widgetry_log::widgetry_error;

/// Field 复用完整 Button，disabled component 只是 root state 的内部镜像。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxField;

/// renderer subtree 的稳定容器，清空 selection 时不影响 Button 与 icon。
#[derive(Component, Default, Clone)]
#[require(FieldProjection)]
pub(crate) struct ComboBoxFieldContent;

/// 从 Popup Visibility 派生 SVG，不保存独立 open state。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxDropdownIcon;

/// 上次成功展开的 Field projection；仅用于判定 rebuild，不参与 selection 决策。
#[derive(Component, Default)]
struct FieldProjection(Option<RenderedSelection>);

/// renderer 同时接收 index 与 value，三个维度任一变化都需要重新展开。
#[derive(Clone, Copy, PartialEq, Eq)]
struct RenderedSelection {
    /// source-local identity，来自内部 ListView state。
    id: WidgetryListItemId,
    /// 当前顺序，move 或其他 entry 的增删都可能改变它。
    index: usize,
    /// 内容版本，与 disabled metadata 无关。
    revision: u64,
}

/// 在 ListView repair 之后派生 Field；不依赖用户 event，不改变 selection 或 Popup。
pub(crate) fn project<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query::<(Entity, &WidgetryComboBox<T>, &Children)>()
        .iter(world)
        .map(|(root, combo, children)| {
            (
                root,
                combo.source(),
                combo.renderer().clone(),
                children.iter().collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    for (root, source, renderer, children) in roots {
        let content = children.iter().find_map(|&field| {
            world.get::<ComboBoxField>(field)?;
            world
                .get::<Children>(field)?
                .iter()
                .find(|&child| world.get::<ComboBoxFieldContent>(child).is_some())
        });
        let content = projection_invariant(content, root);
        let list = children.iter().find_map(|&popup| {
            world.get::<ComboBoxPopup>(popup)?;
            world
                .get::<Children>(popup)?
                .iter()
                .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())
        });
        let list = projection_invariant(list, root);
        let selected =
            projection_invariant(world.get::<WidgetryListViewState>(list), root).selected;
        let model = projection_invariant(world.get::<WidgetryListModel<T>>(source), root);
        let projection = selected.map(|id| {
            let index = projection_invariant(model.index_of(id), root);
            RenderedSelection {
                id,
                index,
                revision: projection_invariant(model.revision(index), root),
            }
        });
        if projection_invariant(world.get::<FieldProjection>(content), root).0 == projection {
            continue;
        }
        let scene = projection.map(|selected| {
            renderer.render(
                selected.index,
                projection_invariant(model.get(selected.index), root),
            )
        });
        let old_children = world
            .get::<Children>(content)
            .map(|children| children.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for child in old_children {
            world.despawn(child);
        }
        if let Some(scene) = scene
            && let Err(error) = world
                .entity_mut(content)
                .apply_scene(bsn! { Children [{scene}] })
        {
            widgetry_error!(?root, ?content, %error, "ComboBox Field renderer Scene 展开失败");
            panic!("ComboBox Field renderer Scene failed");
        }
        world
            .entity_mut(content)
            .insert(FieldProjection(projection));
    }
}

/// 已确认的 ComboBox 缺少必需内部结构或 repair 后的 entry 时属于不可恢复 invariant 错误。
fn projection_invariant<T>(value: Option<T>, root: Entity) -> T {
    value.unwrap_or_else(|| {
        widgetry_error!(
            ?root,
            "ComboBox Field projection 缺少必需内部 state 或 model entry"
        );
        panic!("ComboBox Field projection invariant failed");
    })
}

/// 沿用 ComboBox 的 36px 高度和 10px 水平间距，仅覆盖 Button 几何值。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        ComboBoxField
        @WidgetryButton
        Node {
            width: percent(100), height: px(36),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(10), px(0)),
            border: UiRect::all(px(1)),
        }
        Children [
            (ComboBoxFieldContent Node { align_items: AlignItems::Center }),
            (ComboBoxDropdownIcon
                @WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }
                Node { width: px(16), height: px(16), flex_shrink: 0.0 }),
        ]
    }
}

/// root 新增 disabled state 时同步 Button，并关闭已经展开的 Popup。
pub(crate) fn mirror_disabled_added<T: Send + Sync + 'static>(
    roots: Query<&Children, (With<WidgetryComboBox<T>>, Added<InteractionDisabled>)>,
    fields: Query<(), With<ComboBoxField>>,
    mut popups: Query<&mut Visibility, With<ComboBoxPopup>>,
    mut commands: Commands,
) {
    for children in &roots {
        for child in children.iter() {
            if fields.contains(child) {
                commands.entity(child).insert(InteractionDisabled);
            }
            if let Ok(mut visibility) = popups.get_mut(child) {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// 只处理仍存在且当前已启用的 root，避免同帧移除再插入时覆盖真实 state。
pub(crate) fn mirror_disabled_removed<T: Send + Sync + 'static>(
    mut removed: RemovedComponents<InteractionDisabled>,
    roots: Query<&Children, (With<WidgetryComboBox<T>>, Without<InteractionDisabled>)>,
    fields: Query<(), With<ComboBoxField>>,
    mut commands: Commands,
) {
    for root in removed.read() {
        if let Ok(children) = roots.get(root) {
            for child in children.iter().filter(|&child| fields.contains(child)) {
                commands.entity(child).remove::<InteractionDisabled>();
            }
        }
    }
}

/// Field 在 root 禁用之后才创建时也必须初始化镜像，不向任意 option 内容递归传播。
pub(crate) fn initialize_disabled<T: Send + Sync + 'static>(
    fields: Query<(Entity, &ChildOf), Added<ComboBoxField>>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    for (field, parent) in &fields {
        let Ok(disabled) = roots.get(parent.parent()) else {
            continue;
        };
        if disabled {
            commands.entity(field).insert(InteractionDisabled);
        } else {
            commands.entity(field).remove::<InteractionDisabled>();
        }
    }
}

/// root disabled 新增后立即排队镜像，避免输入 observer 在下一次 PreUpdate 之前改选。
pub(crate) fn on_disabled_added<T: Send + Sync + 'static>(
    event: On<Add, InteractionDisabled>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        queue_list_disabled::<T>(&mut commands, event.entity);
    }
}

/// 移除时按 command 执行后的真实 root state 恢复，兼容同帧移除再插入。
pub(crate) fn on_disabled_removed<T: Send + Sync + 'static>(
    event: On<Remove, InteractionDisabled>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        queue_list_disabled::<T>(&mut commands, event.entity);
    }
}

/// 在当前 mutation 的 deferred queue 中更新内部 ListView，不等待帧级输入派发。
fn queue_list_disabled<T: Send + Sync + 'static>(commands: &mut Commands, root: Entity) {
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryComboBox<T>>(root).is_none() {
            return;
        }
        let disabled = world.get::<InteractionDisabled>(root).is_some();
        let lists = world
            .get::<Children>(root)
            .into_iter()
            .flat_map(|children| children.iter())
            .filter(|&popup| world.get::<ComboBoxPopup>(popup).is_some())
            .flat_map(|popup| {
                world
                    .get::<Children>(popup)
                    .into_iter()
                    .flat_map(|children| children.iter())
            })
            .filter(|&list| world.get::<WidgetryListView<T>>(list).is_some())
            .collect::<Vec<_>>();
        for list in lists {
            if disabled {
                world.entity_mut(list).insert(InteractionDisabled);
            } else {
                world.entity_mut(list).remove::<InteractionDisabled>();
            }
        }
    });
}

/// 内部 ListView 与 Button 一样镜像 root disabled，不改写 model 的 per-item metadata。
pub(crate) fn mirror_list_disabled<T: Send + Sync + 'static>(
    lists: Query<(Entity, &ChildOf, Has<InteractionDisabled>), With<WidgetryListView<T>>>,
    popups: Query<&ChildOf, With<ComboBoxPopup>>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    for (list, parent, list_disabled) in &lists {
        let Ok(popup_parent) = popups.get(parent.parent()) else {
            continue;
        };
        let Ok(disabled) = roots.get(popup_parent.parent()) else {
            continue;
        };
        if disabled != list_disabled {
            if disabled {
                commands.entity(list).insert(InteractionDisabled);
            } else {
                commands.entity(list).remove::<InteractionDisabled>();
            }
        }
    }
}

/// Popup 显隐是箭头方向的唯一来源，WidgetryIcon 自己完成异步 SVG 替换。
pub(crate) fn sync_icon<T: Send + Sync + 'static>(
    popups: Query<(&ChildOf, &Visibility), (With<ComboBoxPopup>, Changed<Visibility>)>,
    roots: Query<&Children, With<WidgetryComboBox<T>>>,
    fields: Query<&Children, With<ComboBoxField>>,
    mut icons: Query<&mut WidgetryIcon, With<ComboBoxDropdownIcon>>,
    server: Res<AssetServer>,
) {
    for (parent, visibility) in &popups {
        let Ok(children) = roots.get(parent.parent()) else {
            continue;
        };
        let icon = children.iter().find_map(|field| {
            fields
                .get(field)
                .ok()?
                .iter()
                .find(|&child| icons.contains(child))
        });
        let Some(icon) = icon else {
            widgetry_error!(root = ?parent.parent(), "ComboBox Field 缺少下拉图标");
            continue;
        };
        if let Ok(mut icon) = icons.get_mut(icon) {
            let path = if *visibility == Visibility::Visible {
                BuiltinIcon::ChevronUp
            } else {
                BuiltinIcon::ChevronDown
            };
            icon.set_svg(&server, path.path());
        }
    }
}
