use crate::assets::GalleryIcon;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{Language, TextKey, localized, tr, tr_args};
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
    label: Option<ItemLabel>,
    edits: usize,
    icon: Option<GalleryIcon>,
}

#[derive(Clone)]
enum ItemLabel {
    Localized(TextKey),
    Raw(String),
}

impl ItemLabel {
    fn display(&self, language: Language, edits: usize) -> String {
        let mut label = match self {
            Self::Localized(key) => tr(language, *key).to_owned(),
            Self::Raw(value) => value.clone(),
        };
        for _ in 0..edits {
            label.push_str(" *");
        }
        label
    }
}

#[derive(Component)]
struct DemoItemText {
    label: ItemLabel,
    edits: usize,
}

fn item_text(label: ItemLabel, edits: usize) -> impl Scene {
    let initial = label.clone();
    bsn! {
        template(move |context| Ok(Text::new(initial.display(*context.resource::<Language>(), edits))))
        template(move |_| Ok(DemoItemText { label: label.clone(), edits }))
        bevy_widgetry::text::WidgetryText
    }
}

fn translate_items(language: Res<Language>, mut texts: Query<(&DemoItemText, &mut Text)>) {
    for (item, mut text) in &mut texts {
        text.set_if_neq(Text::new(item.label.display(*language, item.edits)));
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(crate) struct ComboBoxDemoSources(pub(crate) [Entity; 4]);

pub(crate) struct ComboBoxDemoPlugin;

pub(crate) fn color_examples(world: &World) -> impl Scene + use<> {
    let source = world.resource::<ComboBoxDemoSources>().0[0];
    crate::color_showcase::pair(
        TextKey::ComboBox,
        TextKey::OnlyFieldOpenBorder7C3AedOpenPopup,
        combo(source, "Color Theme"),
        combo(source, "Color Override"),
        |world, entity, apply| {
            use bevy_widgetry::combo_box::WidgetryComboBoxColorOverrides;
            if !apply {
                return WidgetryComboBoxColorOverrides::clear_in_world(world, entity);
            }
            let mut colors = WidgetryComboBoxColorOverrides::default();
            colors.field.open.border = Some(crate::color_showcase::PURPLE);
            WidgetryComboBoxColorOverrides::set_in_world(world, entity, colors)
        },
    )
}

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
        (TextKey::InsertFirst, Action::Insert),
        (TextKey::RemoveSelected, Action::Remove),
        (TextKey::MoveSelectedLast, Action::Move),
        (TextKey::EditSelected, Action::Edit),
        (TextKey::EnableDisableFirst, Action::ToggleDisabled),
        (TextKey::SetSelectedFirst, Action::SelectFirst),
        (TextKey::ClearSelection, Action::ClearSelection),
        (TextKey::ResetModel, Action::Reset),
    ]
    .into_iter()
    .map(|(label, action)| action_button(label, action))
    .collect();
    // Bound generic expansion of the Gallery Scene so nested ComboBox composition cannot exhaust the main thread stack.
    let content: Box<dyn Scene> = Box::new(bsn! {
        template(|_| Ok(ComboBoxPage))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            @localized(TextKey::Renderers) bevy_widgetry::text::WidgetryText--

                Node { flex_direction: FlexDirection::Row, column_gap: px(16), flex_wrap: FlexWrap::Wrap, row_gap: px(8) }
                Children [
                    Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [@localized(TextKey::Text) bevy_widgetry::text::WidgetryText-- @combo(sources[0], "Text")]--
                    Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [@localized(TextKey::IconText) bevy_widgetry::text::WidgetryText-- @combo(sources[1], "Icon + Text")]--
                    Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [@localized(TextKey::Icon) bevy_widgetry::text::WidgetryText-- @combo(sources[2], "Icon")]--
                    Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [@localized(TextKey::Disabled) bevy_widgetry::text::WidgetryText-- @combo(sources[0], "Disabled") InteractionDisabled]
                ]
            --
            @localized(TextKey::DataDriven) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::InsertMoveOrEditSelectionKeepsIts) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }--
            @combo(sources[3], "Data-Driven") template(|_| Ok(DynamicComboBox))--
            template(|_| Ok(DemoStatus::default())) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }--
            Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8) } Children [{controls}]
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
                let text = item.label.as_ref().map(|label| item_text(label.clone(), item.edits));
                bsn_list!{Node { align_items: AlignItems::Center, column_gap: px(6) } Children [{bsn_list!{@icon-- @text}}]}
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

fn action_button(label: TextKey, action: Action) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(DemoAction(action)))
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(operate)
        Children [@localized(label) bevy_widgetry::text::WidgetryText TextFont { font_size: bevy::text::FontSize::Px(14.0) }]
    }
}

fn populate_dynamic(model: &mut WidgetryListModel<ComboBoxDemoItem>) -> Result<(), BevyError> {
    for label in [
        TextKey::Apple,
        TextKey::Banana,
        TextKey::Orange,
        TextKey::Grape,
        TextKey::Pear,
    ] {
        model.push(ComboBoxDemoItem {
            label: Some(ItemLabel::Localized(label)),
            edits: 0,
            icon: None,
        })?;
    }
    model.push(ComboBoxDemoItem {
        label: Some(ItemLabel::Raw("Raw / 自定义".into())),
        icon: None,
        edits: 0,
    })?;
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
                    label: Some(ItemLabel::Localized(TextKey::Inserted)),
                    edits: 0,
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
                item.edits += 1;
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
    language: Res<Language>,
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
        .and_then(|item| {
            item.label
                .as_ref()
                .map(|label| label.display(*language, item.edits))
        })
        .unwrap_or_else(|| tr(*language, TextKey::None).into());
    for (status, mut text) in &mut statuses {
        let value = tr_args(
            *language,
            TextKey::ComboStatus,
            &[
                ("count", model.len().to_string()),
                ("selected", format!("{selected:?}")),
                ("index", format!("{index:?}")),
                ("label", label.clone()),
                (
                    "disabled",
                    model.is_disabled(0).unwrap_or(false).to_string(),
                ),
                ("changes", status.changes.to_string()),
            ],
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
            app.register_type::<ComboBoxDemoSources>()
                .add_systems(
                    PostUpdate,
                    translate_items
                        .run_if(resource_changed::<Language>)
                        .before(bevy::ui::UiSystems::Prepare),
                )
                .add_systems(OnEnter(GalleryPage::ComboBox), enter)
                .add_systems(OnExit(GalleryPage::ComboBox), exit)
                .add_systems(
                    PostUpdate,
                    update_status
                        .after(bevy::ui::UiSystems::Layout)
                        .run_if(in_state(GalleryPage::ComboBox)),
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
    let text = [TextKey::Apple, TextKey::Banana, TextKey::Orange].map(|label| ComboBoxDemoItem {
        label: Some(ItemLabel::Localized(label)),
        edits: 0,
        icon: None,
    });
    let icon_text = [
        (GalleryIcon::ButtonStar, TextKey::Star),
        (GalleryIcon::Logo, TextKey::Logo),
    ]
    .map(|(icon, label)| ComboBoxDemoItem {
        label: Some(ItemLabel::Localized(label)),
        edits: 0,
        icon: Some(icon),
    });
    let icons = [GalleryIcon::ButtonStar, GalleryIcon::Logo].map(|icon| ComboBoxDemoItem {
        label: None,
        edits: 0,
        icon: Some(icon),
    });
    let mut models = std::array::from_fn::<_, 4, _>(|_| WidgetryListModel::default());
    for (model, items) in
        models
            .iter_mut()
            .zip([Vec::from(text), Vec::from(icon_text), Vec::from(icons)])
    {
        for item in items {
            model.push(item)?;
        }
    }
    populate_dynamic(&mut models[3])?;
    let sources = models.map(|model| world.spawn(model).id());
    world.insert_resource(ComboBoxDemoSources(sources));
    if let Err(error) = mount_page(world, GalleryPage::ComboBox, bsn_list! { @scene(sources) }) {
        release_sources(world);
        return Err(error);
    }
    Ok(())
}

fn release_sources(world: &mut World) {
    if let Some(sources) = world
        .get_resource::<ComboBoxDemoSources>()
        .map(|sources| sources.0)
    {
        for source in sources {
            world.despawn(source);
        }
        world.remove_resource::<ComboBoxDemoSources>();
    }
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::ComboBox)?;
    release_sources(world);
    Ok(())
}
