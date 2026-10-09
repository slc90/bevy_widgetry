use crate::assets::GalleryIcon;
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

pub(crate) struct ComboBoxDemoItem {
    label: Option<String>,
    icon: Option<GalleryIcon>,
}

#[derive(Resource)]
pub(crate) struct ComboBoxDemoSources(pub(crate) [Entity; 4]);

pub(crate) struct ComboBoxDemoPlugin;

#[derive(Component)]
struct ComboBoxDemo(&'static str);

#[derive(Component)]
struct ComboBoxPage;

#[derive(Component)]
struct DynamicComboBox;

#[derive(Component, Default)]
struct DemoStatus {
    changes: usize,
}

#[derive(Component)]
struct DemoAction(Action);

#[derive(Clone, Copy, Debug)]
enum Action {
    Insert,
    Remove,
    Move,
    Edit,
    ToggleDisabled,
    SelectFirst,
    ClearSelection,
    Reset,
}

pub(crate) fn scene(sources: [Entity; 4]) -> impl Scene {
    let controls: Vec<_> = [
        ("Insert first", Action::Insert),
        ("Remove selected", Action::Remove),
        ("Move selected → last", Action::Move),
        ("Edit selected", Action::Edit),
        ("Enable/disable first", Action::ToggleDisabled),
        ("Set selected first", Action::SelectFirst),
        ("Clear selection", Action::ClearSelection),
        ("Reset model", Action::Reset),
    ]
    .into_iter()
    .map(|(label, action)| action_button(label, action))
    .collect();
    // 限制整个 Gallery Scene 的泛型展开，避免多层 ComboBox composition 占满主线程 stack。
    let content: Box<dyn Scene> = Box::new(bsn! {
        template(|_| Ok(ComboBoxPage))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            Text("Renderers") bevy_widgetry::text::WidgetryText,
            (
                Node { flex_direction: FlexDirection::Row, column_gap: px(16), flex_wrap: FlexWrap::Wrap, row_gap: px(8) }
                Children [
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Text") bevy_widgetry::text::WidgetryText, combo(sources[0], "Text")]),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Icon + Text") bevy_widgetry::text::WidgetryText, combo(sources[1], "Icon + Text")]),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Icon") bevy_widgetry::text::WidgetryText, combo(sources[2], "Icon")]),
                    (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [Text("Disabled") bevy_widgetry::text::WidgetryText, (combo(sources[0], "Disabled") InteractionDisabled)]),
                ]
            ),
            Text("Data-Driven") bevy_widgetry::text::WidgetryText,
            (Text("Insert, move or edit: selection keeps its identity. Remove selected: Field becomes empty.\nBanana starts disabled. Disabled items stay visible but cannot be selected. Program selection and clear notify on change.") bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }),
            (combo(sources[3], "Data-Driven") template(|_| Ok(DynamicComboBox))),
            (template(|_| Ok(DemoStatus::default())) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }),
            (Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8) } Children [{controls}]),
        ]
    });
    content
}

fn combo(source: Entity, name: &'static str) -> impl Scene {
    bsn! {
        @WidgetryComboBox::<ComboBoxDemoItem> {
            @source: source,
            @renderer: {WidgetryListViewRenderer::new(|_, item: &ComboBoxDemoItem| {
                let icon = item.icon.map(demo_icon);
                let text = item.label.as_ref().map(|label| bsn! { Text({label.clone()}) bevy_widgetry::text::WidgetryText });
                bsn_list![(Node { align_items: AlignItems::Center, column_gap: px(6) } Children [{bsn_list![icon, text]}])]
            })},
        }
        template(move |_| Ok(ComboBoxDemo(name)))
        on(on_selection_changed)
        Node { width: px(200), flex_shrink: 0.0 }
    }
}

fn on_selection_changed(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    demos: Query<&ComboBoxDemo>,
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

fn action_button(label: &'static str, action: Action) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(DemoAction(action)))
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(operate)
        Children [(Text(label) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) })]
    }
}

fn populate_dynamic(model: &mut WidgetryListModel<ComboBoxDemoItem>) -> Result<(), BevyError> {
    for label in ["Apple", "Banana", "Orange", "Grape", "Pear"] {
        model.push(ComboBoxDemoItem {
            label: Some(label.to_owned()),
            icon: None,
        })?;
    }
    model.set_disabled(1, true);
    Ok(())
}

fn operate(
    event: On<Activate>,
    actions: Query<&DemoAction>,
    combos: Query<(Entity, &WidgetryComboBox<ComboBoxDemoItem>), With<DynamicComboBox>>,
    lists: Query<(Entity, &WidgetryListViewState), With<WidgetryListView<ComboBoxDemoItem>>>,
    parents: Query<&ChildOf>,
    mut models: Query<&mut WidgetryListModel<ComboBoxDemoItem>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let Ok(action) = actions.get(event.entity) else {
        return Ok(());
    };
    let Ok((root, combo)) = combos.single() else {
        error!("ComboBox 动态示例缺少唯一 root");
        return Ok(());
    };
    let Ok(mut model) = models.get_mut(combo.source()) else {
        error!(?root, "ComboBox 动态示例缺少 model");
        return Ok(());
    };
    let selected = selected_id(root, &lists, &parents);
    let index = selected.and_then(|id| model.index_of(id));
    let changed = match action.0 {
        Action::Insert => {
            let id = model.insert(
                0,
                ComboBoxDemoItem {
                    label: Some(String::from("Inserted")),
                    icon: None,
                },
            )?;
            info!(item_id = ?id, "插入 ComboBox 示例项");
            true
        }
        Action::Remove => index.is_some_and(|index| model.remove(index).is_some()),
        Action::Move => {
            let last = model.len().saturating_sub(1);
            index.is_some_and(|index| model.move_item(index, last))
        }
        Action::Edit => {
            if let Some(item) = index
                .map(|index| model.get_mut(index))
                .transpose()?
                .flatten()
            {
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
        Action::ClearSelection => {
            WidgetryComboBox::<ComboBoxDemoItem>::clear_selection(&mut commands, root);
            info!(?root, "请求 ComboBox programmatic clear selection");
            return Ok(());
        }
        Action::SelectFirst => {
            if let Some(id) = model.id(0) {
                WidgetryComboBox::<ComboBoxDemoItem>::set_selected(&mut commands, root, id);
                info!(?root, item_id = ?id, "请求 ComboBox programmatic selection");
            } else {
                info!("ComboBox 空 model 无法请求 selection");
            }
            return Ok(());
        }
        Action::Reset => {
            model.clear();
            populate_dynamic(&mut model)?;
            true
        }
    };
    info!(?root, action = ?action.0, ?selected, changed, "执行 ComboBox model 演示操作");
    Ok(())
}

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
            "Items: {} | selected: {:?} | index: {:?} | value: {label}\nFirst item disabled: {} | ValueChange: {}",
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

fn demo_icon(icon: GalleryIcon) -> impl Scene {
    bsn! {
        @WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::new(16, 16))} }
        Node { width: px(16), height: px(16) }
    }
}

impl Plugin for ComboBoxDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            app.register_widgetry_combo_box::<ComboBoxDemoItem>()?;
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
            let mut static_sources = [Entity::PLACEHOLDER; 3];
            for (source, items) in static_sources.iter_mut().zip([
                Vec::from(text),
                Vec::from(icon_text),
                Vec::from(icons),
            ]) {
                let mut model = WidgetryListModel::default();
                for item in items {
                    model.push(item)?;
                }
                *source = app.world_mut().spawn(model).id();
            }
            let mut dynamic = WidgetryListModel::default();
            populate_dynamic(&mut dynamic)?;
            let sources = [
                static_sources[0],
                static_sources[1],
                static_sources[2],
                app.world_mut().spawn(dynamic).id(),
            ];
            app.insert_resource(ComboBoxDemoSources(sources))
                .add_systems(PostUpdate, update_status.after(bevy::ui::UiSystems::Layout));

            Ok(())
        })();
        if let Err(error) = result {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
        }
    }
}
