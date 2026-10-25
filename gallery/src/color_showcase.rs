use crate::i18n::ColorShowcaseKey;
use crate::i18n::{TextKey, localized};
mod catalog;
mod composition;
mod overrides;
mod windows;

use crate::gallery::GalleryPage;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::scene::spawn_scene;
use bevy_widgetry::scroll_area::WidgetryScrollArea;
use bevy_widgetry::text::WidgetryText;
use bevy_widgetry::theme::{WidgetryThemeChanged, WidgetryThemeMode};
use std::collections::BTreeSet;

pub(crate) use overrides::pair;
pub(crate) use overrides::{PURPLE, TEAL};

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut result = Vec::new();
    let mut pending = vec![root];
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
            result.extend(children.iter());
        }
    }
    result
}

pub(crate) struct ColorShowcasePlugin;

#[derive(Component)]
struct Showcase {
    page: GalleryPage,
    section: Section,
    expanded: BTreeSet<String>,
    content: Option<Entity>,
}

#[derive(Clone, Copy, Debug)]
enum Section {
    Catalog,
    Examples,
}

#[derive(Component)]
struct Toggle {
    path: String,
}

pub(crate) fn scene(page: GalleryPage) -> impl Scene {
    let examples = (!matches!(
        page,
        GalleryPage::ProgressBar
            | GalleryPage::Splitter
            | GalleryPage::NumberInput
            | GalleryPage::Canvas
            | GalleryPage::Collapsible
    ))
    .then(|| section(page, Section::Examples));
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), flex_shrink: 0.0, margin: UiRect::bottom(px(12)) }
        Children [@section(page, Section::Catalog)-- @{examples}]
    }
}

fn section(page: GalleryPage, section: Section) -> impl Scene {
    bsn! {
        template(move |_| Ok(Showcase { page, section, expanded: BTreeSet::new(), content: None }))
        Name({format!("Color{page:?}{section:?}")})
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(6) }
        Children [
            @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(match section { Section::Catalog => TextKey::ColorShowcase(ColorShowcaseKey::ExpandThemeColors), Section::Examples => TextKey::ColorShowcase(ColorShowcaseKey::ExpandOverrides) }) WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, } template(|_| Ok(Toggle { path: String::new() }))
                Name({format!("Color{page:?}{section:?}Toggle")})
                Node { align_self: AlignSelf::Start }
                on(toggle)

        ]
    }
}

fn toggle(event: On<Activate>, toggles: Query<&Toggle>, mut commands: Commands) {
    let Ok(toggle) = toggles.get(event.entity) else {
        return;
    };
    let path = toggle.path.clone();
    let button = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        let mut root = button;
        while world.get::<Showcase>(root).is_none() {
            let Some(parent) = world.get::<ChildOf>(root) else { return Ok(()); };
            root = parent.parent();
        }
        if let Some(mut panel) = world.get_mut::<Showcase>(root) {
            if !panel.expanded.remove(&path) { panel.expanded.insert(path.clone()); }
            info!(page = ?panel.page, section = ?panel.section, path, expanded = panel.expanded.contains(&path), "切换颜色展示区域");
        }
        refresh(world, root)
    });
}

fn refresh(world: &mut World, root: Entity) -> Result {
    let Some(panel) = world.get::<Showcase>(root) else {
        return Ok(());
    };
    let page = panel.page;
    let section = panel.section;
    let expanded = panel.expanded.clone();
    let previous = panel.content;
    if let Some(content) = previous
        && let Ok(entity) = world.get_entity_mut(content)
    {
        entity.despawn();
    }
    if let Some(mut panel) = world.get_mut::<Showcase>(root) {
        panel.content = None;
    }
    if !expanded.contains("") {
        return Ok(());
    }
    let content: Box<dyn SceneList> = match section {
        Section::Catalog => catalog::rows(page, *world.resource::<WidgetryThemeMode>(), &expanded),
        Section::Examples => examples(world, page),
    };
    let description = match section {
        Section::Catalog => {
            TextKey::ColorShowcase(ColorShowcaseKey::ThemeSourceValuesPartStateFieldSwatch)
        }
        Section::Examples => {
            TextKey::ColorShowcase(ColorShowcaseKey::RealWidgetsPureThemeOnTheLeft)
        }
    };
    let body = spawn_scene(
        world,
        bsn! {
            ChildOf(root)
            Name({format!("Color{page:?}{section:?}Content")})
            @WidgetryScrollArea {
                @content: bsn! { Node { row_gap: px(8) } },
                @children: bsn_list!{@localized(description) WidgetryText-- {content}},
            }
            Node { width: percent(100), height: px(350), flex_shrink: 0.0 }
        },
    )
    .map_err(|error| {
        error!(?page, ?section, ?root, %error, "颜色展示区域构造失败");
        BevyError::error(error)
    })?;
    if let Some(mut panel) = world.get_mut::<Showcase>(root) {
        panel.content = Some(body);
    }
    Ok(())
}

fn examples(world: &World, page: GalleryPage) -> Box<dyn SceneList> {
    match page {
        GalleryPage::Initializing | GalleryPage::Cursor | GalleryPage::Canvas => {
            Box::new(bsn_list! {})
        }
        GalleryPage::Button => Box::new(bsn_list! {
            @overrides::button_examples()--
            @composition::scene()
        }),
        GalleryPage::Segmented => Box::new(bsn_list! {@crate::pages::segmented_color_examples()}),
        GalleryPage::Tabs => Box::new(bsn_list! {@crate::pages::tabs_color_examples()}),
        GalleryPage::CheckBox => Box::new(bsn_list! {@overrides::check_box_examples()}),
        GalleryPage::ComboBox => Box::new(bsn_list! {@crate::pages::combo_color_examples(world)}),
        GalleryPage::ScrollArea => Box::new(bsn_list! {@overrides::scroll_examples()}),
        GalleryPage::ProgressBar => Box::new(bsn_list! {}),
        GalleryPage::Splitter => Box::new(bsn_list! {}),
        GalleryPage::NumberInput => Box::new(bsn_list! {}),
        GalleryPage::Collapsible => Box::new(bsn_list! {}),
        GalleryPage::Slider => Box::new(bsn_list! {@crate::pages::slider_color_examples()}),
        GalleryPage::ListView => Box::new(bsn_list! {@crate::pages::list_color_examples(world)}),
        GalleryPage::Tree => Box::new(bsn_list! {@crate::pages::tree_color_examples(world)}),
        GalleryPage::Table => Box::new(bsn_list! {@crate::pages::table_color_examples(world)}),
        GalleryPage::TextField => Box::new(bsn_list! {@overrides::text_field_examples()}),
        GalleryPage::Tooltip => Box::new(bsn_list! {@overrides::tooltip_examples()}),
        GalleryPage::Window => Box::new(bsn_list! {@windows::scene()}),
        GalleryPage::Waveform => {
            Box::new(bsn_list! {@crate::pages::waveform_color_examples(world)})
        }
    }
}

fn theme_changed(
    _: On<WidgetryThemeChanged>,
    panels: Query<(Entity, &Showcase)>,
    mut commands: Commands,
) {
    for (root, panel) in &panels {
        if matches!(panel.section, Section::Catalog) && panel.content.is_some() {
            commands.queue(move |world: &mut World| refresh(world, root));
        }
    }
}

impl Plugin for ColorShowcasePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(theme_changed);
    }
}
