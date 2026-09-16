use crate::combo_box::{ComboBoxOptions, WidgetryComboBox};
use crate::option::ComboBoxOption;
use crate::popup::ComboBoxPopup;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_log::widgetry_error;

/// 输入区域复用完整按钮，禁用组件只是root状态的内部镜像。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxField;

/// 保留稳定容器，仅替换选中项 factory 生成的 children。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxFieldContent;

/// 从Popup Visibility 派生 SVG，不保存独立 open 状态。
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxDropdownIcon;

/// 沿用 ComboBox 的 36px 高度和 10px 水平间距，仅覆盖按钮几何值。
pub(crate) fn scene(content: Box<dyn SceneList>) -> impl Scene {
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
            (ComboBoxFieldContent Node { align_items: AlignItems::Center } Children [{content}]),
            (ComboBoxDropdownIcon
                @WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }
                Node { width: px(16), height: px(16), flex_shrink: 0.0 }),
        ]
    }
}

/// 选择组件新增后递归清理旧展示；初始首项副本保留，不依赖用户事件。
pub(crate) fn sync_content(
    selected: Query<(&ComboBoxOption, &ChildOf), Added<Selected>>,
    popups: Query<&ChildOf, With<ComboBoxPopup>>,
    roots: Query<(&ComboBoxOptions, &Children), With<WidgetryComboBox>>,
    fields: Query<&Children, With<ComboBoxField>>,
    contents: Query<(), With<ComboBoxFieldContent>>,
    added_contents: Query<(), Added<ComboBoxFieldContent>>,
    mut commands: Commands,
) {
    for (option, parent) in &selected {
        let Ok(popup_parent) = popups.get(parent.parent()) else {
            widgetry_error!(popup = ?parent.parent(), "ComboBox 选项缺少所属Popup");
            continue;
        };
        let root = popup_parent.parent();
        let Ok((options, children)) = roots.get(root) else {
            widgetry_error!(?root, "ComboBox Popup缺少有效root和选项来源");
            continue;
        };
        let Some(factory) = options.0.get(option.index) else {
            widgetry_error!(
                ?root,
                option_index = option.index,
                option_count = options.0.len(),
                "ComboBox 内部选项索引越界"
            );
            continue;
        };
        let content = children.iter().find_map(|field| {
            fields
                .get(field)
                .ok()?
                .iter()
                .find(|&child| contents.contains(child))
        });
        let Some(content) = content else {
            widgetry_error!(?root, "ComboBox 缺少 Field 内容容器");
            continue;
        };
        // Scene 已构造首项副本；只跳过初始首项同步，首帧前改选仍需重建。
        if option.index == 0 && added_contents.contains(content) {
            continue;
        }
        let scene = factory.build();
        commands.entity(content).despawn_children();
        commands.queue(move |world: &mut World| -> Result {
            world
                .entity_mut(content)
                .apply_scene(bsn! { Children [{scene}] })?;
            Ok(())
        });
    }
}

/// root新增禁用状态时同步按钮，并关闭已经展开的Popup。
pub(crate) fn mirror_disabled_added(
    roots: Query<&Children, (With<WidgetryComboBox>, Added<InteractionDisabled>)>,
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

/// 只处理仍存在且当前已启用的root，避免同帧移除再插入时覆盖真实状态。
pub(crate) fn mirror_disabled_removed(
    mut removed: RemovedComponents<InteractionDisabled>,
    roots: Query<&Children, (With<WidgetryComboBox>, Without<InteractionDisabled>)>,
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

/// Field 在root禁用之后才创建时也必须初始化镜像，不向任意选项内容递归传播。
pub(crate) fn initialize_disabled(
    fields: Query<(Entity, &ChildOf), Added<ComboBoxField>>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryComboBox>>,
    mut commands: Commands,
) {
    for (field, parent) in &fields {
        let Ok(disabled) = roots.get(parent.parent()) else {
            widgetry_error!(?field, "ComboBox Field 缺少所属root");
            continue;
        };
        if disabled {
            commands.entity(field).insert(InteractionDisabled);
        } else {
            commands.entity(field).remove::<InteractionDisabled>();
        }
    }
}

/// Popup 显隐是箭头方向的唯一来源，WidgetryIcon 自己完成异步 SVG 替换。
pub(crate) fn sync_icon(
    popups: Query<(&ChildOf, &Visibility), (With<ComboBoxPopup>, Changed<Visibility>)>,
    roots: Query<&Children, With<WidgetryComboBox>>,
    fields: Query<&Children, With<ComboBoxField>>,
    mut icons: Query<&mut WidgetryIcon, With<ComboBoxDropdownIcon>>,
    server: Res<AssetServer>,
) {
    for (parent, visibility) in &popups {
        let Ok(children) = roots.get(parent.parent()) else {
            widgetry_error!(root = ?parent.parent(), "ComboBox Popup缺少所属root");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WidgetryComboBoxPlugin;
    use bevy_widgetry_test_utils::scene_app;

    // root早已禁用且本帧不再 Added 时，新创建的内部按钮仍必须镜像root的当前状态。
    #[test]
    fn field_added_after_disabled_root_initializes_mirror() {
        let mut app = scene_app();
        app.add_plugins(WidgetryComboBoxPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                WidgetryComboBox Node InteractionDisabled
            })
            .unwrap()
            .id();
        app.update();
        let field = app
            .world_mut()
            .spawn_scene(bsn! {
                scene(Box::new(bsn_list![Text("Option")]))
                template(move |_| Ok(ChildOf(root)))
            })
            .unwrap()
            .id();
        app.update();
        assert!(app.world().get::<InteractionDisabled>(field).is_some());
        assert_eq!(
            app.world().get::<BackgroundColor>(field).unwrap().0,
            bevy_widgetry_core::DARK_THEME.control_background_disabled
        );
    }
}
