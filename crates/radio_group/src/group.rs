use crate::WidgetryRadioOption;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::{RadioGroup, ValueChange};
use bevy_widgetry_log::widgetry_error;

/// 通过 BSN 构造的标准 RadioGroup，需注册 WidgetryRadioGroupPlugin。
/// root 自带 TabIndex；调用方必须在 ancestor UI root 配置 Bevy 的
/// [TabGroup](bevy::input_focus::tab_navigation::TabGroup)，才能通过 Tab 进入 Group。
/// TabGroup 不应放在 Group 自身；输入 plugin 与派发设施由应用提供，详见 WidgetryRadioGroupPlugin。
/// direct children 必须为非空且固定顺序的 WidgetryRadioOption，初始化默认选中 index 0。
/// 用户改选在 root 发出 ValueChange\<usize>；disabled 只需设置在 root。
/// 初始化会拒绝非法 hierarchy，先记录 ERROR 再 panic；不支持初始化后增删或重排 options。
/// 首次 PreUpdate 完成初始化与 disabled 镜像；颜色由 theme 管理，layout 可通过 Node patch。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryRadioGroup;

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
pub(crate) fn initialize(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, (With<WidgetryRadioGroup>, Without<Initialized>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        initialize_group(world, root);
    }
    let options = world
        .query_filtered::<(Entity, Option<&ChildOf>), Added<WidgetryRadioOption>>()
        .iter(world)
        .map(|(child, parent)| (child, parent.map(ChildOf::parent)))
        .collect::<Vec<_>>();
    for (child, root) in options {
        if !root.is_some_and(|root| world.get::<WidgetryRadioGroup>(root).is_some()) {
            if let Some(root) = root {
                widgetry_error!(
                    ?root,
                    ?child,
                    "RadioOption 必须是 RadioGroup 的 direct child"
                );
            } else {
                widgetry_error!(?child, "RadioOption 缺少 RadioGroup parent");
            }
            panic!("WidgetryRadioOption requires a direct WidgetryRadioGroup parent");
        }
    }
}

/// 校验不可恢复的构造错误后建立默认 selection；程序化 API 也可在首帧前完成此步骤。
fn initialize_group(world: &mut World, root: Entity) {
    if world.get::<Initialized>(root).is_some() {
        return;
    }
    let children = world
        .get::<Children>(root)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    if children.is_empty() {
        widgetry_error!(?root, "RadioGroup 至少需要一个 option");
        panic!("WidgetryRadioGroup requires at least one option");
    }
    for &child in &children {
        if world.get::<WidgetryRadioOption>(child).is_none() {
            widgetry_error!(
                ?root,
                ?child,
                "RadioGroup 的 direct child 必须是 RadioOption"
            );
            panic!("WidgetryRadioGroup requires WidgetryRadioOption children");
        }
    }
    select(world, &children, children[0]);
    world.entity_mut(root).insert(Initialized);
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
    /// 静默设置 direct child index；无效 entity、越界或同值为 no-op，disabled 时仍允许。
    /// 不发送 ValueChange\<Entity> 或 ValueChange\<usize>，style 在后续 Update 同步。
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: usize) {
        commands.queue(move |world: &mut World| {
            if world.get::<WidgetryRadioGroup>(entity).is_none() {
                return;
            }
            initialize_group(world, entity);
            let Some(children) = world
                .get::<Children>(entity)
                .map(|children| children.to_vec())
            else {
                return;
            };
            let Some(&target) = children.get(selected) else {
                return;
            };
            if world.get::<Checked>(target).is_some() {
                return;
            }
            select(world, &children, target);
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
