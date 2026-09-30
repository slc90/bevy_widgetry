use crate::assets::GalleryIcon;
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry::style::{ForegroundColor, ThemeChanged, ThemeMode};

/// 示例 model 的业务内容，与 UI hierarchy 分开存活。
pub(crate) struct ComboBoxDemoItem {
    /// 可选文本，纯 icon 示例不提供文本。
    label: Option<String>,
    /// Gallery 自有 icon，纯文本示例不提供 icon。
    icon: Option<GalleryIcon>,
}

/// 保存静态 renderer 示例和动态示例的独立 source；root disabled 示例共享文本 model。
#[derive(Resource)]
pub(crate) struct ComboBoxDemoSources(pub(crate) [Entity; 4]);

/// 注册业务 type 并维护 Gallery 的 model 操作与公开 selection 展示。
pub(crate) struct ComboBoxDemoPlugin;

/// 区分 root 的示例名称，selection 通过 model-local id 表达。
#[derive(Component)]
struct ComboBoxDemo(&'static str);

/// 页面自有说明和 status 的 theme 传播边界。
#[derive(Component)]
struct ComboBoxPage;

/// 标识可操作的 ComboBox，避免修改静态 renderer 和 TitleBar 示例。
#[derive(Component)]
struct DynamicComboBox;

/// 仅统计动态示例的用户通知，不存储第二份 selection。
#[derive(Component, Default)]
struct DemoStatus {
    /// programmatic selection 与 model CRUD 不增加此计数。
    changes: usize,
}

/// 将真实 Button 的 Activate 映射到 Gallery model 操作。
#[derive(Component)]
struct DemoAction(Action);

/// 动态示例需要展示的公开 CRUD、stable selection 与 disabled 能力。
#[derive(Clone, Copy, Debug)]
enum Action {
    Insert,
    Remove,
    Move,
    Edit,
    ToggleDisabled,
    SelectFirst,
    Reset,
}

/// 保留四种 renderer 示例，独立动态 model 展示 ComboBox 的 identity 与内容同步。
pub(crate) fn scene(sources: [Entity; 4]) -> impl Scene {
    let controls: Vec<_> = [
        ("Insert first", Action::Insert),
        ("Remove selected", Action::Remove),
        ("Move selected → last", Action::Move),
        ("Edit selected", Action::Edit),
        ("Enable/disable first", Action::ToggleDisabled),
        ("Set selected first", Action::SelectFirst),
        ("Reset model", Action::Reset),
    ]
    .into_iter()
    .map(|(label, action)| action_button(label, action))
    .collect();
    // 限制整个 Gallery Scene 的泛型展开，避免多层 ComboBox composition 占满主线程 stack。
    let content: Box<dyn Scene> = Box::new(bsn! {
        template(|_| Ok(ComboBoxPage))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            Text("Renderers"),
            (
                Node { flex_direction: FlexDirection::Row, column_gap: px(16), flex_wrap: FlexWrap::Wrap, row_gap: px(8) }
                Children [
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Text"), combo(sources[0], "Text")]),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Icon + Text"), combo(sources[1], "Icon + Text")]),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Icon"), combo(sources[2], "Icon")]),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Disabled"), (combo(sources[0], "Disabled") InteractionDisabled)]),
                ]
            ),
            Text("Data-Driven"),
            (Text("Insert, move or edit: selection keeps its identity. Remove selected: Field becomes empty.\nBanana starts disabled. Disabled items stay visible but cannot be selected. Set selected first is silent.") TextFont { font_size: bevy::text::FontSize::Px(14.0) }),
            (combo(sources[3], "Data-Driven") template(|_| Ok(DynamicComboBox))),
            (template(|_| Ok(DemoStatus::default())) Text("") TextFont { font_size: bevy::text::FontSize::Px(14.0) }),
            (Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8) } Children [{controls}]),
        ]
    });
    content
}

/// 使用同一个业务 renderer 构造列表，保留 Gallery 内容的自主字体与 icon 配置。
fn combo(source: Entity, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryComboBox::<ComboBoxDemoItem> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, item: &ComboBoxDemoItem| {
                let icon = item.icon.map(demo_icon);
                let text = item.label.as_ref().map(|label| bsn! { Text({label.clone()}) });
                bsn_list![(Node { align_items: AlignItems::Center, column_gap: px(6) } Children [{bsn_list![icon, text]}])]
            })},
        }
        template(move |_| Ok(ComboBoxDemo(name)))
        on(on_selection_changed)
        Node { width: px(200), flex_shrink: 0.0 }
    }
}

/// root 通知使用 stable id；初始化与程序化改选仍然静默。
fn on_selection_changed(
    event: On<ValueChange<WidgetryListItemId>>,
    demos: Query<&ComboBoxDemo, Without<InteractionDisabled>>,
    mut statuses: Query<&mut DemoStatus>,
) {
    if let Ok(demo) = demos.get(event.source) {
        info!(demo = demo.0, item_id = ?event.value, "选择 ComboBox 示例项");
        if demo.0 == "Data-Driven" {
            for mut status in &mut statuses {
                status.changes += 1;
            }
        }
    }
}

/// 通过公开 ListView state 与 ancestor hierarchy 读取指定 ComboBox 的真实 selection。
fn selected_id(
    root: Entity,
    lists: &Query<(Entity, &WidgetryListViewState), With<WidgetryListView<ComboBoxDemoItem>>>,
    parents: &Query<&ChildOf>,
) -> Option<WidgetryListItemId> {
    lists.iter().find_map(|(list, state)| {
        parents
            .iter_ancestors(list)
            .any(|ancestor| ancestor == root)
            .then_some(state.selected)
            .flatten()
    })
}

/// 操作按钮只使用公开 WidgetryButton composition，不触碰 Popup 内部。
fn action_button(label: &'static str, action: Action) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(DemoAction(action)))
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(operate)
        Children [(Text(label) TextFont { font_size: bevy::text::FontSize::Px(14.0) })]
    }
}

/// model reset 保留独立 source 和单调 id counter，第二项初始 disabled。
fn populate_dynamic(model: &mut WidgetryListModel<ComboBoxDemoItem>) {
    for label in ["Apple", "Banana", "Orange", "Grape", "Pear"] {
        model.push(ComboBoxDemoItem {
            label: Some(label.to_owned()),
            icon: None,
        });
    }
    model.set_disabled(1, true);
}

/// 根据当前 stable id 操作业务 model；删除后的空 selection 由 ListView repair 和 Field projection 收敛。
fn operate(
    event: On<Activate>,
    actions: Query<&DemoAction>,
    combos: Query<(Entity, &WidgetryComboBox<ComboBoxDemoItem>), With<DynamicComboBox>>,
    lists: Query<(Entity, &WidgetryListViewState), With<WidgetryListView<ComboBoxDemoItem>>>,
    parents: Query<&ChildOf>,
    mut models: Query<&mut WidgetryListModel<ComboBoxDemoItem>>,
    mut commands: Commands,
) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let Ok((root, combo)) = combos.single() else {
        error!("ComboBox 动态示例缺少唯一 root");
        return;
    };
    let Ok(mut model) = models.get_mut(combo.source()) else {
        error!(?root, "ComboBox 动态示例缺少 model");
        return;
    };
    let selected = selected_id(root, &lists, &parents);
    let index = selected.and_then(|id| model.index_of(id));
    let changed = match action.0 {
        Action::Insert => match model.insert(
            0,
            ComboBoxDemoItem {
                label: Some(String::from("Inserted")),
                icon: None,
            },
        ) {
            Ok(id) => {
                info!(item_id = ?id, "插入 ComboBox 示例项");
                true
            }
            Err(_) => {
                error!("ComboBox 动态示例在合法 index 插入失败");
                return;
            }
        },
        Action::Remove => index.is_some_and(|index| model.remove(index).is_some()),
        Action::Move => {
            let last = model.len().saturating_sub(1);
            index.is_some_and(|index| model.move_item(index, last))
        }
        Action::Edit => {
            if let Some(item) = index.and_then(|index| model.get_mut(index)) {
                if let Some(label) = &mut item.label {
                    label.push_str(" *");
                }
                true
            } else {
                false
            }
        }
        Action::ToggleDisabled => model
            .is_disabled(0)
            .is_some_and(|disabled| model.set_disabled(0, !disabled)),
        Action::SelectFirst => {
            if let Some(id) = model.id(0) {
                WidgetryComboBox::<ComboBoxDemoItem>::set_selected(&mut commands, root, id);
                info!(?root, item_id = ?id, "请求 ComboBox programmatic selection");
            } else {
                info!("ComboBox 空 model 无法请求 selection");
            }
            return;
        }
        Action::Reset => {
            model.clear();
            populate_dynamic(&mut model);
            true
        }
    };
    info!(?root, action = ?action.0, ?selected, changed, "执行 ComboBox model 演示操作");
}

/// 文字直接派生自公开 model 和真实 selection，显示 CRUD 对 identity/index 与通知计数的影响。
fn update_status(
    combos: Query<(Entity, &WidgetryComboBox<ComboBoxDemoItem>), With<DynamicComboBox>>,
    lists: Query<(Entity, &WidgetryListViewState), With<WidgetryListView<ComboBoxDemoItem>>>,
    parents: Query<&ChildOf>,
    models: Query<&WidgetryListModel<ComboBoxDemoItem>>,
    mut statuses: Query<(&DemoStatus, &mut Text)>,
) {
    let Ok((root, combo)) = combos.single() else {
        return;
    };
    let Ok(model) = models.get(combo.source()) else {
        return;
    };
    let selected = selected_id(root, &lists, &parents);
    let index = selected.and_then(|id| model.index_of(id));
    let label = selected
        .and_then(|id| model.get_by_id(id))
        .and_then(|item| item.label.as_deref())
        .unwrap_or("<none>");
    for (status, mut text) in &mut statuses {
        let value = format!(
            "Items: {} | selected: {:?} | index: {:?} | value: {label}\nFirst item disabled: {} | User ValueChange: {}",
            model.len(),
            selected,
            index,
            model.is_disabled(0).unwrap_or(false),
            status.changes
        );
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// 页面自有标题、说明和 status 随全局 theme 刷新；Widget 内容由库管理。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut pages: Query<&mut Propagate<ForegroundColor>, With<ComboBoxPage>>,
) {
    for mut foreground in &mut pages {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

/// 使用 Gallery 自有 asset，并继承 wrapper 的 foreground color。
fn demo_icon(icon: GalleryIcon) -> impl Scene {
    bsn! {
        @WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::new(16, 16))} }
        Node { width: px(16), height: px(16) }
    }
}

impl Plugin for ComboBoxDemoPlugin {
    fn build(&self, app: &mut App) {
        app.register_widgetry_combo_box::<ComboBoxDemoItem>();
        let text = ["Apple", "Banana", "Orange"].map(|label| ComboBoxDemoItem {
            label: Some(label.to_owned()),
            icon: None,
        });
        let icon_text = [
            (GalleryIcon::ButtonStar, "Star"),
            (GalleryIcon::Logo, "Logo"),
        ]
        .map(|(icon, label)| ComboBoxDemoItem {
            label: Some(label.to_owned()),
            icon: Some(icon),
        });
        let icons = [GalleryIcon::ButtonStar, GalleryIcon::Logo].map(|icon| ComboBoxDemoItem {
            label: None,
            icon: Some(icon),
        });
        let static_sources =
            [Vec::from(text), Vec::from(icon_text), Vec::from(icons)].map(|items| {
                let mut model = WidgetryListModel::default();
                for item in items {
                    model.push(item);
                }
                app.world_mut().spawn(model).id()
            });
        let mut dynamic = WidgetryListModel::default();
        populate_dynamic(&mut dynamic);
        let sources = [
            static_sources[0],
            static_sources[1],
            static_sources[2],
            app.world_mut().spawn(dynamic).id(),
        ];
        app.insert_resource(ComboBoxDemoSources(sources))
            .add_observer(refresh_theme)
            .add_systems(PostUpdate, update_status.after(bevy::ui::UiSystems::Layout));
    }
}
