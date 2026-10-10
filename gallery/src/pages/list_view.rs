use crate::gallery::{GalleryPage, mount_page, unmount_page};
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewAppExt,
    WidgetryListViewItem, WidgetryListViewPlugin, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry::scroll_area::WidgetryScrollAreaViewport;

pub(crate) struct ListViewDemoPlugin;

pub(crate) fn color_examples(world: &World) -> impl Scene + use<> {
    let source = world.resource::<DemoSources>().0[0];
    let view = move || {
        bsn! {
            @WidgetryListView::<DemoItem> {
                @source: source, @item_height: 32.0,
                @renderer: {WidgetryListViewRenderer::new(|_, value: &DemoItem| bsn_list!{Text({value.label.clone()}) bevy_widgetry::text::WidgetryText})},
            }
            Node { width: percent(100), height: px(160) }
        }
    };
    crate::color_showcase::pair(
        "ListView",
        "仅 item.selected.background=#0F766E；选中后移开 pointer，active border 不变。",
        view(),
        view(),
        |world, entity, apply| {
            use bevy_widgetry::list_view::WidgetryListViewColorOverrides;
            if !apply {
                return WidgetryListViewColorOverrides::clear_in_world(world, entity);
            }
            let mut colors = WidgetryListViewColorOverrides::default();
            colors.item.selected.background = Some(crate::color_showcase::TEAL);
            WidgetryListViewColorOverrides::set_in_world(world, entity, colors)
        },
    )
}

struct DemoItem {
    label: String,
    edits: usize,
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(crate) struct DemoSources(pub(crate) [Entity; 4]);

#[derive(Component)]
struct ListViewDemo;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct DemoKind(usize);

#[derive(Component, Default)]
struct DemoStatus {
    changes: usize,
}

#[derive(Component, Clone, Copy)]
struct DemoAction(Action);

#[derive(Clone, Copy, Debug)]
enum Action {
    Select,
    ClearSelection,
    VisibleUpdate,
    OffscreenUpdate,
    Insert,
    Remove,
    Move,
    Reset,
    Shrink,
    Scroll,
    ToggleDisabled,
    Resize,
}

pub(crate) fn scene(sources: [Entity; 4]) -> impl Scene {
    bsn! {
        template(|_| Ok(ListViewDemo))
        Node {
            width: percent(100), height: percent(100), display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.0)],
            grid_template_rows: vec![RepeatedGridTrack::flex(2, 1.0)],
            column_gap: px(24), row_gap: px(16),
        }
        Children [
            @section(sources[0], 0, "Small List", "Arrow/Home/End: active; Space/Enter: select. Click text or row.")--
            @section(sources[1], 1, "Virtualized Large List", "10,000 items; wheel/PageUp/PageDown; jump, edit, shrink and resize.")--
            @section(sources[2], 2, "Disabled ListView", "User input blocked; model, selection and scroll buttons remain available.")--
            @section(sources[3], 3, "Disabled ListViewItem", "Initially items 1/3 disabled. Arrow/click can activate; Space/Enter cannot select.")
        ]
    }
}

fn section(
    source: Entity,
    kind: usize,
    title: &'static str,
    description: &'static str,
) -> impl Scene {
    let controls: Vec<_> = [
        ("Set selected", Action::Select),
        ("Clear selection", Action::ClearSelection),
        ("Edit visible", Action::VisibleUpdate),
        ("Insert first", Action::Insert),
        ("Remove first", Action::Remove),
        ("Move first → last", Action::Move),
        ("Clear + repopulate", Action::Reset),
    ]
    .into_iter()
    .chain(match kind {
        1 => vec![
            ("Edit #7500", Action::OffscreenUpdate),
            ("Shrink to 3", Action::Shrink),
            ("Resize viewport", Action::Resize),
        ],
        2 => vec![("Scroll +96px", Action::Scroll)],
        3 => vec![
            ("Enable/disable #1", Action::ToggleDisabled),
            ("Scroll +96px", Action::Scroll),
        ],
        _ => vec![],
    })
    .map(|(label, action)| action_button(label, action))
    .collect();
    bsn! {
        template(move |_| Ok(DemoKind(kind)))
        Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text(title) bevy_widgetry::text::WidgetryText--
            Text(description) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }--

                template(move |_| Ok(DemoKind(kind)))
                template(|_| Ok(DemoStatus::default()))
                Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }
            --

                @WidgetryListView::<DemoItem> {
                    @source: source,
                    @item_height: 32.0,
                    @renderer: {WidgetryListViewRenderer::new(|_, value: &DemoItem| {
                        let label = format!("{}  (edits: {})", value.label, value.edits);
                        bsn_list!{Node { width: percent(100) } Children [Text({label}) bevy_widgetry::text::WidgetryText]}
                    })},
                }
                template(move |_| Ok(DemoKind(kind)))
                template(move |_| Ok(Name::new(format!("ListViewDemo{kind}"))))
                Node { width: percent(100), height: px(162), flex_shrink: 0.0 }
                on(record_change)
            --
            Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8) } Children [{controls}]
        ]
    }
}

fn action_button(label: &'static str, action: Action) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(DemoAction(action)))
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(operate)
        Children [Text(label) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }]
    }
}

fn populate(model: &mut WidgetryListModel<DemoItem>, kind: usize) -> Result<(), BevyError> {
    let len = match kind {
        0 => 4,
        1 => 10_000,
        _ => 20,
    };
    for index in 0..len {
        model.push(DemoItem {
            label: format!("Item {index}"),
            edits: 0,
        })?;
    }
    if kind == 3 {
        model.set_disabled(1, true);
        model.set_disabled(3, true);
    }
    Ok(())
}

fn belongs(entity: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    entity == root
        || parents
            .iter_ancestors(entity)
            .any(|ancestor| ancestor == root)
}

fn operate(
    event: On<Activate>,
    actions: Query<&DemoAction>,
    kinds: Query<&DemoKind>,
    parents: Query<&ChildOf>,
    mut lists: Query<(Entity, &DemoKind, &WidgetryListView<DemoItem>, &mut Node)>,
    mut models: Query<&mut WidgetryListModel<DemoItem>>,
    rows: Query<(Entity, &WidgetryListViewItem)>,
    mut viewports: Query<(Entity, &mut ScrollPosition), With<WidgetryScrollAreaViewport>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let Ok(action) = actions.get(event.entity) else {
        return Ok(());
    };
    let Some(kind) = parents
        .iter_ancestors(event.entity)
        .find_map(|ancestor| kinds.get(ancestor).ok())
    else {
        return Ok(());
    };
    let Some((root, _, view, mut node)) = lists
        .iter_mut()
        .find(|(_, candidate, _, _)| *candidate == kind)
    else {
        return Ok(());
    };
    let Ok(mut model) = models.get_mut(view.source()) else {
        return Ok(());
    };
    info!(entity = ?root, action = ?action.0, "执行 ListView programmatic 演示操作" );
    match action.0 {
        Action::ClearSelection => {
            WidgetryListView::<DemoItem>::clear_selection(&mut commands, root)
        }
        Action::Select => WidgetryListView::<DemoItem>::set_selected(
            &mut commands,
            root,
            match kind.0 {
                1 => 7500,
                2 => 10,
                _ => 1,
            },
        ),
        Action::VisibleUpdate | Action::OffscreenUpdate => {
            let index = if matches!(action.0, Action::OffscreenUpdate) {
                Some(7500)
            } else {
                rows.iter()
                    .filter(|(entity, _)| belongs(*entity, root, &parents))
                    .map(|(_, item)| item.index)
                    .min()
            };
            if let Some(value) = index
                .map(|index| model.get_mut(index))
                .transpose()?
                .flatten()
            {
                value.edits += 1;
            }
        }
        Action::Insert => {
            model.insert(
                0,
                DemoItem {
                    label: "Inserted item".into(),
                    edits: 0,
                },
            )?;
        }
        Action::Remove => {
            model.remove(0);
        }
        Action::Move => {
            if !model.is_empty() {
                let last = model.len() - 1;
                model.move_item(0, last);
            }
        }
        Action::Reset => {
            model.clear();
            populate(&mut model, kind.0)?;
        }
        Action::Shrink => {
            while model.len() > 3 {
                let last = model.len() - 1;
                model.remove(last);
            }
        }
        Action::Scroll => {
            for (entity, mut position) in &mut viewports {
                if belongs(entity, root, &parents) {
                    position.0.y += 96.0;
                }
            }
        }
        Action::ToggleDisabled => {
            if let Some(disabled) = model.is_disabled(1) {
                model.set_disabled(1, !disabled);
            }
        }
        Action::Resize => {
            node.height = if node.height == px(162) {
                px(258)
            } else {
                px(162)
            };
        }
    }
    Ok(())
}

fn record_change(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    lists: Query<&DemoKind, With<WidgetryListView<DemoItem>>>,
    mut statuses: Query<(&DemoKind, &mut DemoStatus)>,
) {
    info!(entity = ?event.source, selected = ?event.value, "ListView selection 已改变" );
    let Ok(kind) = lists.get(event.source) else {
        return;
    };
    for (candidate, mut status) in &mut statuses {
        if candidate == kind {
            status.changes += 1;
        }
    }
}

fn update_status(
    lists: Query<(
        Entity,
        &DemoKind,
        &WidgetryListView<DemoItem>,
        &WidgetryListViewState,
    )>,
    models: Query<&WidgetryListModel<DemoItem>>,
    rows: Query<(Entity, &WidgetryListViewItem)>,
    parents: Query<&ChildOf>,
    mut statuses: Query<(&DemoKind, &DemoStatus, &mut Text)>,
) {
    for (root, kind, view, state) in &lists {
        let Ok(model) = models.get(view.source()) else {
            continue;
        };
        let mut indices: Vec<_> = rows
            .iter()
            .filter(|(entity, _)| belongs(*entity, root, &parents))
            .map(|(_, item)| item.index)
            .collect();
        indices.sort_unstable();
        let start = indices.first().copied().unwrap_or(0);
        let end = indices.last().map_or(0, |index| index + 1);
        for (candidate, status, mut text) in &mut statuses {
            if candidate != kind {
                continue;
            }
            let value = format!(
                "Model items: {} | Rendered rows: {} | Rendered range: {start}..{end}\nselected: {:?} / {:?} | active: {:?} / {:?} | ValueChange: {}",
                model.len(),
                indices.len(),
                state.selected,
                state.selected.and_then(|id| model.index_of(id)),
                state.active,
                state.active.and_then(|id| model.index_of(id)),
                status.changes
            );
            if text.0 != value {
                text.0 = value;
            }
        }
    }
}

fn initialize_disabled(
    mut commands: Commands,
    lists: Query<(Entity, &DemoKind), Added<WidgetryListView<DemoItem>>>,
) {
    for (entity, kind) in &lists {
        if kind.0 == 2 {
            commands.entity(entity).insert(InteractionDisabled);
            WidgetryListView::<DemoItem>::set_selected(&mut commands, entity, 1);
        }
    }
}

impl Plugin for ListViewDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            if !app.is_plugin_added::<WidgetryListViewPlugin>() {
                app.add_plugins(WidgetryListViewPlugin);
            }
            app.register_widgetry_list_view::<DemoItem>()?;
            app.register_type::<DemoSources>()
                .add_systems(OnEnter(GalleryPage::ListView), enter)
                .add_systems(OnExit(GalleryPage::ListView), exit)
                .add_systems(
                    Update,
                    initialize_disabled.run_if(in_state(GalleryPage::ListView)),
                )
                .add_systems(
                    PostUpdate,
                    update_status
                        .after(bevy::ui::UiSystems::Layout)
                        .run_if(in_state(GalleryPage::ListView)),
                );

            Ok(())
        })();
        if let Err(error) = result {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
        }
    }
}

fn enter(world: &mut World) -> Result {
    let mut models = std::array::from_fn::<_, 4, _>(|_| WidgetryListModel::default());
    for (kind, model) in models.iter_mut().enumerate() {
        populate(model, kind)?;
    }
    let sources = models.map(|model| world.spawn(model).id());
    world.insert_resource(DemoSources(sources));
    if let Err(error) = mount_page(world, GalleryPage::ListView, bsn_list! { @scene(sources) }) {
        release_sources(world);
        return Err(error);
    }
    Ok(())
}

fn release_sources(world: &mut World) {
    if let Some(sources) = world.get_resource::<DemoSources>().map(|sources| sources.0) {
        for source in sources {
            world.despawn(source);
        }
        world.remove_resource::<DemoSources>();
    }
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::ListView)?;
    release_sources(world);
    Ok(())
}
