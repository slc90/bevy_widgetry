use crate::WidgetryRadioOption;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{RadioGroup, ValueChange};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// 通过 BSN 构造的标准 RadioGroup，需注册 WidgetryRadioGroupPlugin。
/// root 自带 TabIndex；调用方必须在 ancestor UI root 配置 Bevy 的
/// [TabGroup](bevy::input_focus::tab_navigation::TabGroup)，才能通过 Tab 进入 Group。
/// TabGroup 不应放在 Group 自身；输入 plugin 与派发设施由应用提供，详见 WidgetryRadioGroupPlugin。
/// direct children 必须为非空且固定顺序的 WidgetryRadioOption，初始化默认选中 index 0。
/// 用户改选在 root 发出 ValueChange\<usize>；disabled 只需设置在 root。
/// 初始化会拒绝非法 hierarchy，记录 ERROR 并上抛 BevyError；不支持初始化后增删或重排 options。
/// 首次 PreUpdate 完成初始化与 disabled 镜像；颜色由 theme 管理，layout 可通过 Node patch。
///
/// selection authority 仅在 options 的官方 Checked，不另存 root selection。
/// root 的 On<ValueChange<usize>> 提供固定 direct child index，并保留官方 is_final。
/// 同一操作还可能发出 option 的 ValueChange<bool> 与 Group 的 ValueChange<Entity>；
/// App 级 consumer 应按 type / source 选择通知，避免重复统计。
/// radio_self_update 排队维护 Checked，index 转发不保证通知前全部 option state 已更新；使用 event.value 识别目标。
/// 默认初始化与程序 set_selected 静默；重复选中同一 option 不发改选通知。
/// root disabled 镜像到 options，不提供单项 disabled 契约。
#[derive(SceneComponent, Default, Clone)]
#[require(GroupDiagnostics)]
pub struct WidgetryRadioGroup;

/// 初始化异常由 root 持有，持续失败仍上抛但不逐帧重复日志。
#[derive(Component, Default)]
struct GroupDiagnostics(FailureState);

/// direct parent 校验的异常边界随 option 生命周期销毁。
#[derive(Component, Default)]
pub(crate) struct OwnershipDiagnostics(FailureState);

/// 记录默认 selection 已建立，避免覆盖首帧前的显式 set_selected。
#[derive(Component)]
struct Initialized;

/// 仅把本 Widget 的 direct child 转成公开 index；Checked 完全交由 radio_self_update 更新。
pub(crate) fn handle_value_change(
    event: On<ValueChange<Entity>>,
    groups: Query<&Children, With<WidgetryRadioGroup>>,
    mut commands: Commands,
) {
    let Ok(children) = groups.get(event.source) else {
        return;
    };
    if let Some(index) = children.iter().position(|child| child == event.value) {
        commands.trigger(ValueChange {
            source: event.source,
            value: index,
            is_final: event.is_final,
        });
    }
}

/// 首帧在完整 BSN hierarchy 上静默建立默认 selection。
pub(crate) fn initialize(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, (With<WidgetryRadioGroup>, Without<Initialized>)>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = initialize_group(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    let options = world
        .query_filtered::<(Entity, Option<&ChildOf>), With<WidgetryRadioOption>>()
        .iter(world)
        .map(|(child, parent)| (child, parent.map(ChildOf::parent)))
        .collect::<Vec<_>>();
    for (child, root) in options {
        let result = if root.is_some_and(|root| world.get::<WidgetryRadioGroup>(root).is_some()) {
            Ok(())
        } else {
            Err(BevyError::error(
                "WidgetryRadioOption requires a direct WidgetryRadioGroup parent",
            ))
        };
        let result = if let Some(mut diagnostics) = world.get_mut::<OwnershipDiagnostics>(child) {
            diagnostics.0.observe(
                result,
                |error| match root {
                    Some(root) => {
                        widgetry_error!(?root, ?child, %error, "RadioOption parent 校验失败")
                    }
                    None => widgetry_error!(?child, %error, "RadioOption parent 校验失败"),
                },
                || widgetry_info!(?root, ?child, "RadioOption parent 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// 校验不可恢复的构造错误后建立默认 selection；程序化 API 也可在首帧前完成此步骤。
fn initialize_group(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let result = initialize_group_inner(world, root);
    let invalid_child = world.get::<Children>(root).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryRadioOption>(*child).is_none())
    });
    if let Some(mut diagnostics) = world.get_mut::<GroupDiagnostics>(root) {
        diagnostics.0.observe(
            result,
            |error| match invalid_child {
                Some(child) => widgetry_error!(?root, ?child, %error, "RadioGroup 初始化失败"),
                None => widgetry_error!(?root, %error, "RadioGroup 初始化失败"),
            },
            || widgetry_info!(?root, "RadioGroup 初始化恢复正常"),
        )
    } else {
        result
    }
}

/// 成功后才写入 Initialized；错误不能留下部分默认 selection。
fn initialize_group_inner(world: &mut World, root: Entity) -> Result<(), BevyError> {
    if world.get::<Initialized>(root).is_some() {
        return Ok(());
    }
    let children = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    if children.is_empty() {
        return Err(BevyError::error(
            "WidgetryRadioGroup requires at least one option",
        ));
    }
    for &child in &children {
        if world.get::<WidgetryRadioOption>(child).is_none() {
            return Err(BevyError::error(format!(
                "WidgetryRadioGroup requires WidgetryRadioOption children: {child:?}"
            )));
        }
    }
    select(world, &children, children[0]);
    world.entity_mut(root).insert(Initialized);
    Ok(())
}

/// 仅供初始化与程序化选择使用；用户互斥选择由官方 observer 处理。
fn select(world: &mut World, children: &[Entity], target: Entity) {
    for &child in children {
        if child == target {
            world.entity_mut(child).insert(Checked);
        } else {
            world.entity_mut(child).remove::<Checked>();
        }
    }
}

/// 在输入派发前镜像 root disabled；仅访问 direct option，不触碰用户内容。
pub(crate) fn mirror_disabled(
    changed: Query<
        Entity,
        (
            With<WidgetryRadioGroup>,
            Or<(Added<WidgetryRadioGroup>, Added<InteractionDisabled>)>,
        ),
    >,
    groups: Query<(&Children, Has<InteractionDisabled>), With<WidgetryRadioGroup>>,
    options: Query<(), With<WidgetryRadioOption>>,
    mut removed: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    for root in changed.iter().chain(removed.read()) {
        let Ok((children, disabled)) = groups.get(root) else {
            continue;
        };
        for child in children.iter().filter(|&child| options.contains(child)) {
            if disabled {
                commands.entity(child).insert(InteractionDisabled);
            } else {
                commands.entity(child).remove::<InteractionDisabled>();
            }
        }
    }
}

impl WidgetryRadioGroup {
    /// queue 执行时静默设置固定 direct child index；disabled 时仍允许。
    /// 不发送 ValueChange\<Entity> 或 ValueChange\<usize>，style 在后续 Update 同步。
    /// 无效 root 被忽略；首次调用先校验 hierarchy 并建立默认 index 0，再应用有效 index。
    /// 首帧前的有效设置不会被后续初始化覆盖；首次越界仍可能建立默认 selection。
    /// 初始化后越界或同值为 no-op；非法构造 hierarchy 记录 ERROR 并交给宿主 error handler。
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: usize) {
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            if world.get::<WidgetryRadioGroup>(entity).is_none() {
                return Ok(());
            }
            initialize_group(world, entity)?;
            let Some(children) = world
                .get::<Children>(entity)
                .map(|children| children.to_vec())
            else {
                return Ok(());
            };
            let Some(&target) = children.get(selected) else {
                return Ok(());
            };
            if world.get::<Checked>(target).is_some() {
                return Ok(());
            }
            select(world, &children, target);
            Ok(())
        });
    }

    /// 展开 Group 的官方行为与默认 layout，允许调用方 patch Node。
    fn scene() -> impl Scene {
        bsn! {
            RadioGroup
            TabIndex::default()
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: px(6),
                padding: UiRect::all(px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)),
            }
            BackgroundColor
            BorderColor
        }
    }
}
