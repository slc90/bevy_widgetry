use crate::model::contract_error;
use crate::*;
use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::scene::{apply_scene, spawn_scene};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use std::ops::Range;
use std::sync::{Arc, Weak};

#[derive(Component, Clone)]
pub(crate) struct FileDialogEntryViewport {
    viewport: Entity,
    content: Entity,
    top: Entity,
    bottom: Entity,
    rows: Vec<Entity>,
    generation: Option<(WidgetryFileDialogSessionId, u64)>,
    query_revision: u64,
    active: Option<WidgetryFileDialogEntryId>,
    count: usize,
    snapshot: Option<Weak<WidgetryFileDialogSnapshot>>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EntryRow {
    pub(crate) root: Entity,
    pub(crate) id: WidgetryFileDialogEntryId,
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) index: usize,
    pub(crate) snapshot_identity: usize,
}

#[derive(Component)]
struct RenderedStyle {
    font: f32,
    entry: WidgetryFileDialogEntry,
}

fn range(
    len: usize,
    height: f32,
    viewport: f32,
    offset: f32,
    overscan: usize,
) -> (f32, Range<usize>) {
    if !viewport.is_finite() || viewport <= 0.0 {
        return (0.0, 0..0);
    }
    let maximum = (len as f32 * height - viewport).max(0.0);
    let offset = if offset.is_finite() {
        offset.clamp(0.0, maximum)
    } else {
        0.0
    };
    let start = ((offset / height).floor() as usize)
        .saturating_sub(overscan)
        .min(len);
    let end = (((offset + viewport) / height).ceil() as usize)
        .saturating_add(overscan)
        .min(len);
    (offset, start..end)
}

fn required<T>(value: Option<T>) -> Result<T> {
    value.ok_or_else(|| contract_error("FileDialog entry viewport structure missing"))
}

pub(crate) fn snapshot_identity(state: &WidgetryFileDialogState) -> Option<usize> {
    // view 与 press 保存 Weak，保证其 allocation 地址不被复用。
    // Weak 不保留 snapshot payload，最后的大数据析构仍由后台 ownership 决定。
    state
        .snapshot()
        .map(|snapshot| Arc::as_ptr(snapshot) as usize)
}

fn bootstrap(world: &mut World, area: Entity, multiple: bool) -> Result<FileDialogEntryViewport> {
    let viewport = required(world.get::<Children>(area).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryScrollAreaViewport>(*child).is_some())
    }))?;
    let content = required(world.get::<Children>(viewport).and_then(|children| {
        children
            .iter()
            .find(|child| world.get::<WidgetryScrollAreaContent>(*child).is_some())
    }))?;
    apply_scene(&mut world.entity_mut(content), bsn! {
        Children [
            (Name("FileDialogTopSpacer") Node { height: px(0), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))),
            (Name("FileDialogBottomSpacer") Node { height: px(0), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))),
        ]
    }).map_err(|error| contract_error(&error.to_string()))?;
    let children = required(world.get::<Children>(content))?;
    let top = required(children.iter().next())?;
    let bottom = required(children.iter().nth(1))?;
    let mut accessibility = accesskit::Node::new(accesskit::Role::ListBox);
    if multiple {
        accessibility.set_multiselectable();
    }
    world
        .entity_mut(area)
        .insert(AccessibilityNode(accessibility));
    world.write_message(bevy::window::RequestRedraw);
    Ok(FileDialogEntryViewport {
        viewport,
        content,
        top,
        bottom,
        rows: Vec::new(),
        generation: None,
        query_revision: 0,
        active: None,
        count: 0,
        snapshot: None,
    })
}

fn row_content(entry: &WidgetryFileDialogEntry, font: f32) -> impl SceneList {
    let icon = if entry.kind().is_directory() {
        BuiltinIcon::FileDialogFolder
    } else {
        BuiltinIcon::FileDialogFile
    };
    let size = entry
        .size()
        .map(|size| format!("{size} B"))
        .unwrap_or_else(|| "—".into());
    bsn_list![
        (@WidgetryIcon { @path: {icon.path()}, @max_size: {Some(UVec2::splat(16))} } Node { width: px(16), height: px(16), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))),
        (Text({entry.name().to_string_lossy().into_owned()}) bevy_widgetry_core::text::WidgetryText TextFont { font_size: font } Node { flex_grow: 1.0, min_width: px(0), overflow: Overflow::clip() } template(|_| Ok(Pickable::IGNORE))),
        (Text(size) bevy_widgetry_core::text::WidgetryText TextFont { font_size: font } Node { width: px(90), justify_content: JustifyContent::FlexEnd } template(|_| Ok(Pickable::IGNORE))),
    ]
}

pub(crate) fn page_size(world: &World, area: Entity) -> usize {
    world
        .get::<FileDialogEntryViewport>(area)
        .and_then(|view| {
            let height = world.get::<ComputedNode>(view.viewport)?;
            let part = world.get::<crate::style::Part>(area)?;
            let style = world.get::<WidgetryFileDialogStyle>(part.root)?;
            Some(
                ((height.size().y * height.inverse_scale_factor()) / style.row_height)
                    .floor()
                    .max(1.0) as usize,
            )
        })
        .unwrap_or(1)
}

pub(crate) fn reconcile(
    world: &mut World,
    root: Entity,
    area: Entity,
    state: &WidgetryFileDialogState,
    style: &WidgetryFileDialogStyle,
) -> Result {
    let mut view = world
        .get::<FileDialogEntryViewport>(area)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| bootstrap(world, area, state.mode().is_multiple()))?;
    let computed = required(world.get::<ComputedNode>(view.viewport))?;
    let height = computed.size().y * computed.inverse_scale_factor();
    let mut offset = required(world.get::<ScrollPosition>(view.viewport))?.0.y;
    let generation = (state.token().session, state.token().generation);
    if view.generation != Some(generation)
        && state.navigation != WidgetryFileDialogNavigation::Refresh
    {
        offset = 0.0;
    }
    let len = state.visible().len();
    if (state.active() != view.active
        || view.count != len
        || view.query_revision != state.token().query_revision)
        && let Some(index) = state.active().and_then(|id| {
            state
                .snapshot()
                .and_then(|snapshot| snapshot.positions.get(&id).copied())
        })
    {
        let top = index as f32 * style.row_height;
        if top < offset {
            offset = top;
        } else if top + style.row_height > offset + height {
            offset = top + style.row_height - height;
        }
    }
    let (offset, visible) = range(len, style.row_height, height, offset, style.overscan);
    if visible.len() > 256 {
        return Err(contract_error("FileDialog visible row capacity exceeded"));
    }
    let mut scroll = required(world.get_mut::<ScrollPosition>(view.viewport))?;
    if scroll.0.y != offset {
        scroll.0.y = offset;
    }
    let mut rows = Vec::with_capacity(visible.len());
    let mut changed = view.rows.len() != visible.len();
    for (slot, index) in visible.clone().enumerate() {
        let id = required(state.visible().get(index).copied())?;
        let entry = required(state.snapshot().and_then(|snapshot| snapshot.entry(id)))?;
        let row = EntryRow {
            root,
            id,
            token: state.token(),
            index,
            snapshot_identity: required(snapshot_identity(state))?,
        };
        let previous = view.rows.get(slot).copied();
        let entity = match previous {
            Some(entity) => {
                let identity_changed = world.get::<EntryRow>(entity).is_none_or(|old| old.id != id);
                if identity_changed
                    || world
                        .get::<RenderedStyle>(entity)
                        .is_none_or(|old| old.font != style.font_size || old.entry != *entry)
                {
                    let old_children: Vec<_> = world
                        .get::<Children>(entity)
                        .map(|children| children.iter().collect())
                        .unwrap_or_default();
                    for child in old_children {
                        world.despawn(child);
                    }
                    apply_scene(
                        &mut world.entity_mut(entity),
                        bsn! { Children [{row_content(entry, style.font_size)}] },
                    )
                    .map_err(|error| contract_error(&error.to_string()))?;
                    changed = true;
                }
                entity
            }
            None => {
                changed = true;
                spawn_scene(world, bsn! {
                    Name("FileDialogEntryRow") BackgroundColor::default() BorderColor::default()
                    template(|_| Ok(ResolvedForeground::default()))
                    Node { width: percent(100), align_items: AlignItems::Center, flex_shrink: 0.0, column_gap: px(8), padding: UiRect::horizontal(px(8)), border: UiRect::all(px(1)) }
                    Children [{row_content(entry, style.font_size)}]
                }).map_err(|error| contract_error(&error.to_string()))?
            }
        };
        let selected = state.selected().contains(&id);
        let mut accessibility = accesskit::Node::new(accesskit::Role::ListBoxOption);
        accessibility.set_label(entry.name().to_string_lossy().into_owned());
        accessibility.set_selected(selected);
        if world.get::<EntryRow>(entity) != Some(&row) {
            world.entity_mut(entity).insert(row);
        }
        if world
            .get::<RenderedStyle>(entity)
            .is_none_or(|old| old.font != style.font_size || old.entry != *entry)
        {
            world.entity_mut(entity).insert(RenderedStyle {
                font: style.font_size,
                entry: entry.clone(),
            });
        }
        if let Some(mut value) = world.get_mut::<AccessibilityNode>(entity) {
            if value.0 != accessibility {
                value.0 = accessibility;
            }
        } else {
            world
                .entity_mut(entity)
                .insert(AccessibilityNode(accessibility));
        }
        let mut node = required(world.get_mut::<Node>(entity))?;
        if node.height != px(style.row_height) {
            node.height = px(style.row_height);
            node.min_height = px(style.row_height);
            node.max_height = px(style.row_height);
        }
        rows.push(entity);
    }
    for entity in view.rows.iter().skip(rows.len()) {
        world.despawn(*entity);
    }
    for (entity, extent) in [
        (view.top, visible.start as f32 * style.row_height),
        (view.bottom, (len - visible.end) as f32 * style.row_height),
    ] {
        let mut node = required(world.get_mut::<Node>(entity))?;
        if node.height != px(extent) {
            node.height = px(extent);
        }
    }
    let mut order = Vec::with_capacity(rows.len() + 2);
    order.push(view.top);
    order.extend(rows.iter().copied());
    order.push(view.bottom);
    if world
        .get::<Children>(view.content)
        .is_none_or(|children| !children.iter().eq(order.iter().copied()))
    {
        world.entity_mut(view.content).replace_children(&order);
    }
    view.rows = rows;
    view.generation = Some(generation);
    view.query_revision = state.token().query_revision;
    view.active = state.active();
    view.count = len;
    if view.snapshot.as_ref().map(Weak::as_ptr) != state.snapshot().map(Arc::as_ptr) {
        view.snapshot = state.snapshot().map(Arc::downgrade);
    }
    world.entity_mut(area).insert(view);
    if changed {
        world.write_message(bevy::window::RequestRedraw);
    }
    Ok(())
}

pub(crate) fn update_entry_colors(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryFileDialogColors,
) -> Result<(), BevyError> {
    let Some(state) = world.get::<WidgetryFileDialogState>(root).cloned() else {
        return Ok(());
    };
    let rows = world
        .query::<(Entity, &EntryRow)>()
        .iter(world)
        .filter(|(_, r)| r.root == root)
        .map(|(e, r)| (e, *r))
        .collect::<Vec<_>>();
    for (entity, row) in rows {
        let disabled = world.get::<bevy::ui::InteractionDisabled>(entity).is_some();
        let state_colors = if disabled {
            colors.entry.disabled
        } else if state.selected().contains(&row.id) {
            colors.entry.selected
        } else {
            colors.entry.normal
        };
        let border = if state.active() == Some(row.id) {
            if disabled {
                colors.entry.disabled_active_border
            } else {
                colors.entry.active_border
            }
        } else {
            state_colors.border
        };
        required(world.get_mut::<BackgroundColor>(entity))?
            .set_if_neq(BackgroundColor(state_colors.background));
        required(world.get_mut::<BorderColor>(entity))?.set_if_neq(BorderColor::all(border));
        required(world.get_mut::<ResolvedForeground>(entity))?
            .set_if_neq(ResolvedForeground(state_colors.foreground));
    }
    Ok(())
}

// visible range 的边界断言属于测试 contract，允许使用测试 macros。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::range;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn visible_range_is_clamped_and_bounded(len in 0usize..100_001, height in 1f32..100f32, viewport in 1f32..2000f32, offset in -1_000_000f32..20_000_000f32, overscan in 0usize..9) {
            let (offset, visible) = range(len, height, viewport, offset, overscan);
            prop_assert!(offset >= 0.0 && offset <= (len as f32 * height - viewport).max(0.0));
            prop_assert!(visible.start <= visible.end && visible.end <= len);
            prop_assert!(visible.len() <= (viewport / height).ceil() as usize + overscan * 2 + 1);
        }
    }

    #[test]
    fn nonfinite_scroll_and_unlaid_out_viewport_do_not_materialize_all_entries() {
        assert_eq!(range(100_000, 28.0, 0.0, f32::NAN, 2), (0.0, 0..0));
        assert_eq!(range(100_000, 28.0, 100.0, f32::INFINITY, 2), (0.0, 0..6));
    }
}
