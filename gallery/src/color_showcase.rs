mod catalog;
mod composition;
mod overrides;
mod windows;

use crate::gallery::GalleryPage;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::scene::WidgetrySceneCommandsExt;
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
    bsn! {
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(8), flex_shrink: 0.0, margin: UiRect::bottom(px(12)) }
        Children [@section(page, Section::Catalog, "Theme 颜色")-- @section(page, Section::Examples, "显式覆盖")]
    }
}

fn section(page: GalleryPage, section: Section, label: &'static str) -> impl Scene {
    bsn! {
        template(move |_| Ok(Showcase { page, section, expanded: BTreeSet::new(), content: None }))
        Name({format!("Color{page:?}{section:?}")})
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(6) }
        Children [
            @WidgetryButton template(|_| Ok(Toggle { path: String::new() }))
                Name({format!("Color{page:?}{section:?}Toggle")})
                Node { align_self: AlignSelf::Start }
                on(toggle)
                Children [Text({format!("展开 / 折叠 {label}")}) WidgetryText]
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
            "Theme 原值目录：部件 / state / 字段 / 色块 / HEX（含 alpha），不是控件最终颜色。"
        }
        Section::Examples => {
            "真实控件：左侧纯 Theme，右侧只覆盖指定字段。应用 / 清除通过公开 API；清除回到当前 Theme。"
        }
    };
    let body = world
        .commands()
        .spawn_scene_with_error_handler(bsn! {
            ChildOf(root)
            Name({format!("Color{page:?}{section:?}Content")})
            @WidgetryScrollArea {
                @content: bsn! { Node { row_gap: px(8) } },
                @children: bsn_list!{Text(description) WidgetryText-- {content}},
            }
            Node { width: percent(100), height: px(350), flex_shrink: 0.0 }
        })
        .id();
    if let Some(mut panel) = world.get_mut::<Showcase>(root) {
        panel.content = Some(body);
    }
    Ok(())
}

fn examples(world: &World, page: GalleryPage) -> Box<dyn SceneList> {
    match page {
        GalleryPage::Button => Box::new(bsn_list! {
            @overrides::button_examples()--
            @composition::scene()
        }),
        GalleryPage::CheckBox => Box::new(bsn_list! {@overrides::check_box_examples()}),
        GalleryPage::ComboBox => Box::new(bsn_list! {@crate::pages::combo_color_examples(world)}),
        GalleryPage::ScrollArea => Box::new(bsn_list! {@overrides::scroll_examples()}),
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
