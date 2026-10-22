use crate::colors::{ColorState, TabColorState};
use crate::tabs::{Parts, State, TabOwner, TabParts};
use crate::{TabsOrientation, WidgetryTab, WidgetryTabs};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{LineBreak, TextLayoutInfo};
use bevy::ui::{Checked, InteractionDisabled, Pressed, UiSystems};
use bevy_widgetry_core::{
    diagnostics::FailureState, foreground::ResolvedForeground, ui::WidgetryUiSystems,
};
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

pub(crate) type FrameOwner = bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTabs>;
pub(crate) type LabelOwner = bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTab>;

#[derive(Component, Default)]
pub(crate) struct StyleDiagnostics(FailureState);

#[derive(Resource, Default)]
struct Dirty {
    colors: EntityHashSet,
    geometry: EntityHashSet,
}

// Cache Bevy's complete shaped output, never a second copy of the title's source Text.
// The tick distinguishes a native reflow from our own rendering projection.
#[derive(Component)]
struct ElidedLayout {
    source: TextLayoutInfo,
    projected_at: bevy::ecs::change_detection::Tick,
}

#[derive(Component)]
struct DirectTitleWidth(Val);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Dirty>()
        .add_observer(
            |_: On<WidgetryThemeChanged>,
             entities: Query<Entity, Or<(With<WidgetryTabs>, With<WidgetryTab>)>>,
             mut dirty: ResMut<Dirty>| {
                dirty.colors.extend(entities.iter());
            },
        )
        .add_observer(
            |event: On<
                Remove<(
                    Checked,
                    Hovered,
                    Pressed,
                    InteractionDisabled,
                    ColorState,
                    TabColorState,
                    BackgroundColor,
                    BorderColor,
                    ResolvedForeground,
                )>,
            >,
             frames: Query<&FrameOwner>,
             labels: Query<&LabelOwner>,
             mut dirty: ResMut<Dirty>| {
                let target = frames
                    .get(event.entity)
                    .map(|owner| owner.entity)
                    .or_else(|_| labels.get(event.entity).map(|owner| owner.entity))
                    .unwrap_or(event.entity);
                dirty.colors.insert(target);
            },
        )
        .add_systems(
            PostUpdate,
            configure_labels.in_set(WidgetryUiSystems::Materialize),
        )
        .add_systems(
            PostUpdate,
            (collect_colors, update_colors)
                .chain()
                .in_set(WidgetryUiSystems::Colors),
        )
        .add_systems(
            PostUpdate,
            (measure_direct_titles, collect_geometry, update_ellipsis)
                .chain()
                .after(UiSystems::PostLayout),
        );
}

fn collect_colors(
    roots: Query<
        (Entity, &State),
        (
            With<WidgetryTabs>,
            Or<(Changed<ColorState>, Changed<State>, Added<Parts>)>,
        ),
    >,
    headers: Query<
        Entity,
        (
            With<WidgetryTab>,
            Or<(
                Changed<TabColorState>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<Checked>,
                Added<InteractionDisabled>,
                Added<TabOwner>,
                Added<BackgroundColor>,
                Added<BorderColor>,
                Added<ResolvedForeground>,
            )>,
        ),
    >,
    outputs: Query<
        (Option<&FrameOwner>, Option<&LabelOwner>),
        (
            Or<(With<FrameOwner>, With<LabelOwner>)>,
            Or<(
                Added<BackgroundColor>,
                Added<BorderColor>,
                Added<ResolvedForeground>,
            )>,
        ),
    >,
    focus: Res<InputFocus>,
    mut previous: Local<Option<Entity>>,
    mut dirty: ResMut<Dirty>,
) {
    for (root, state) in &roots {
        dirty.colors.insert(root);
        dirty
            .colors
            .extend(state.pairs.iter().map(|pair| pair.header));
    }
    dirty.colors.extend(headers.iter());
    for (frame, label) in &outputs {
        if let Some(owner) = frame {
            dirty.colors.insert(owner.entity);
        }
        if let Some(owner) = label {
            dirty.colors.insert(owner.entity);
        }
    }
    if focus.is_changed() {
        dirty.colors.extend(*previous);
        dirty.colors.extend(focus.get());
        *previous = focus.get();
    }
}

fn update_colors(world: &mut World) -> Result<(), BevyError> {
    let dirty = std::mem::take(&mut world.resource_mut::<Dirty>().colors);
    let mut failure = None;
    for entity in dirty {
        if world.get::<WidgetryTabs>(entity).is_none() && world.get::<TabOwner>(entity).is_none() {
            continue;
        }
        let result = apply(world, entity);
        let result = if let Some(mut diagnostics) = world.get_mut::<StyleDiagnostics>(entity) {
            diagnostics.0.observe(
                result,
                |error| widgetry_error!(?entity, %error, "Tabs 样式结构失效"),
                || widgetry_info!(?entity, "Tabs 样式恢复正常"),
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
    failure.map_or(Ok(()), Err)
}

fn apply(world: &mut World, entity: Entity) -> Result<(), BevyError> {
    let root = world
        .get::<TabOwner>(entity)
        .map(|owner| owner.root)
        .unwrap_or(entity);
    let state = world
        .get::<State>(root)
        .ok_or_else(|| BevyError::error("Tabs state is missing"))?;
    if state.pairs.is_empty() {
        return Ok(());
    }
    let orientation = state.orientation;
    let colors = world
        .get::<ColorState>(root)
        .ok_or_else(|| BevyError::error("Tabs colors are missing"))?
        .0
        .resolve(&world.resource::<WidgetryThemeMode>().colors().tabs);
    if entity == root {
        let parts = world
            .get::<Parts>(root)
            .ok_or_else(|| BevyError::error("Tabs parts are missing"))?;
        let (bar, panel) = (parts.bar, parts.pages);
        if world.get::<BackgroundColor>(bar).is_none()
            || world.get::<BackgroundColor>(panel).is_none()
            || world.get::<BorderColor>(panel).is_none()
        {
            return Err(BevyError::error(
                "Tabs frame is missing managed style outputs",
            ));
        }
        if let Some(mut output) = world.get_mut::<BackgroundColor>(bar) {
            output.set_if_neq(BackgroundColor(colors.bar_background));
        }
        if let Some(mut output) = world.get_mut::<BackgroundColor>(panel) {
            output.set_if_neq(BackgroundColor(colors.panel_background));
        }
        if let Some(mut output) = world.get_mut::<BorderColor>(panel) {
            output.set_if_neq(BorderColor::all(colors.separator));
        }
    } else {
        let colors = world
            .get::<TabColorState>(entity)
            .ok_or_else(|| BevyError::error("Tab colors are missing"))?
            .0
            .resolve(&colors.header);
        let selected = world.get::<Checked>(entity).is_some();
        let disabled = world.get::<InteractionDisabled>(entity).is_some();
        let interactions = if selected {
            colors.selected
        } else {
            colors.unselected
        };
        let colors_for_state = if disabled {
            interactions.disabled
        } else if world.get::<Pressed>(entity).is_some() {
            interactions.pressed
        } else if world
            .get::<Hovered>(entity)
            .is_some_and(|hovered| hovered.0)
        {
            interactions.hovered
        } else {
            interactions.normal
        };
        let mut border = BorderColor::all(
            if !disabled && world.resource::<InputFocus>().get() == Some(entity) {
                colors.focus_border
            } else {
                colors_for_state.border
            },
        );
        if selected {
            match orientation {
                TabsOrientation::Horizontal => border.bottom = colors_for_state.background,
                TabsOrientation::Vertical => border.right = colors_for_state.background,
            }
        }
        let ellipsis = world
            .get::<TabParts>(entity)
            .ok_or_else(|| BevyError::error("Tab parts are missing"))?
            .ellipsis;
        if world.get::<BackgroundColor>(entity).is_none()
            || world.get::<BorderColor>(entity).is_none()
            || world.get::<ResolvedForeground>(entity).is_none()
            || world.get::<ResolvedForeground>(ellipsis).is_none()
        {
            return Err(BevyError::error("Tab is missing managed style outputs"));
        }
        if let Some(mut output) = world.get_mut::<BackgroundColor>(entity) {
            output.set_if_neq(BackgroundColor(colors_for_state.background));
        }
        if let Some(mut output) = world.get_mut::<BorderColor>(entity) {
            output.set_if_neq(border);
        }
        if let Some(mut output) = world.get_mut::<ResolvedForeground>(entity) {
            output.set_if_neq(ResolvedForeground(colors_for_state.foreground));
        }
        if let Some(mut output) = world.get_mut::<ResolvedForeground>(ellipsis) {
            output.set_if_neq(ResolvedForeground(colors_for_state.foreground));
        }
    }
    Ok(())
}

fn header_ancestor(
    entity: Entity,
    parents: &Query<&ChildOf>,
    headers: &Query<&TabParts, With<TabOwner>>,
) -> Option<Entity> {
    let mut entity = entity;
    loop {
        if let Ok(parts) = headers.get(entity) {
            return (parts.ellipsis != entity).then_some(entity);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}

fn configure_labels(
    changed: Query<
        Entity,
        Or<(
            Added<TabOwner>,
            Changed<Text>,
            Changed<TextLayout>,
            Changed<ChildOf>,
        )>,
    >,
    headers: Query<&TabParts, With<TabOwner>>,
    parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let roots: EntityHashSet = changed
        .iter()
        .filter_map(|entity| header_ancestor(entity, &parents, &headers))
        .collect();
    for header in roots {
        commands.queue(move |world: &mut World| {
            let Some(owner) = world.get::<TabOwner>(header) else {
                return;
            };
            if world.get::<State>(owner.root).is_none() {
                return;
            }
            let ellipsis = world.get::<TabParts>(header).map(|parts| parts.ellipsis);
            let mut pending = vec![header];
            while let Some(entity) = pending.pop() {
                if Some(entity) == ellipsis {
                    continue;
                }
                if entity != header && world.get::<bevy::ui::OverrideClip>(entity).is_some() {
                    continue;
                }
                if world.get::<Text>(entity).is_some()
                    && world.get::<bevy::text::EditableText>(entity).is_none()
                    && let Some(mut layout) = world.get_mut::<TextLayout>(entity)
                    && layout.linebreak != LineBreak::NoWrap
                {
                    layout.linebreak = LineBreak::NoWrap;
                }
                if let Some(children) = world.get::<Children>(entity) {
                    pending.extend(children.iter());
                }
            }
        });
    }
}

fn measure_direct_titles(
    mut headers: Query<
        (
            Entity,
            &TabOwner,
            &ComputedNode,
            &TextLayoutInfo,
            &mut Node,
            Option<&DirectTitleWidth>,
        ),
        (
            With<Text>,
            Without<bevy::text::EditableText>,
            Or<(
                Changed<ComputedNode>,
                Changed<TextLayoutInfo>,
                Changed<Node>,
            )>,
        ),
    >,
    roots: Query<&State>,
    mut commands: Commands,
) {
    for (entity, owner, computed, layout, mut node, previous) in &mut headers {
        if roots
            .get(owner.root)
            .ok()
            .is_none_or(|state| state.orientation != TabsOrientation::Horizontal)
            || node.width != Val::Auto
            || layout.size.x == 0.0
        {
            continue;
        }
        if node.min_width != previous.map_or(px(0), |previous| previous.0) {
            continue;
        }
        let width = px((layout.size.x
            + computed.padding.min_inset.x
            + computed.padding.max_inset.x
            + computed.border.min_inset.x
            + computed.border.max_inset.x)
            * computed.inverse_scale_factor);
        if node.min_width != width {
            node.min_width = width;
            commands.entity(entity).insert(DirectTitleWidth(width));
        }
    }
}

fn collect_geometry(
    changed: Query<Entity, Or<(Changed<ComputedNode>, Changed<UiGlobalTransform>)>>,
    layouts: Query<(Entity, Ref<TextLayoutInfo>, Option<&ElidedLayout>), Changed<TextLayoutInfo>>,
    headers: Query<&TabParts, With<TabOwner>>,
    parents: Query<&ChildOf>,
    mut dirty: ResMut<Dirty>,
) {
    dirty.geometry.extend(
        changed
            .iter()
            .filter_map(|entity| header_ancestor(entity, &parents, &headers)),
    );
    dirty
        .geometry
        .extend(layouts.iter().filter_map(|(entity, layout, elided)| {
            if elided.is_some_and(|elided| elided.projected_at == layout.last_changed()) {
                return None;
            }
            header_ancestor(entity, &parents, &headers)
        }));
}

fn update_ellipsis(world: &mut World) {
    let dirty = std::mem::take(&mut world.resource_mut::<Dirty>().geometry);
    for header in dirty {
        let Some(owner) = world.get::<TabOwner>(header) else {
            continue;
        };
        let vertical = world
            .get::<State>(owner.root)
            .is_some_and(|state| state.orientation == TabsOrientation::Vertical);
        let Some(parts) = world.get::<TabParts>(header) else {
            continue;
        };
        let ellipsis = parts.ellipsis;
        let mut overflow = false;
        let mut font = None;
        let mut texts = Vec::new();
        if vertical
            && let (Some(node), Some(transform)) = (
                world.get::<ComputedNode>(header),
                world.get::<UiGlobalTransform>(header),
            )
        {
            let right = transform.translation.x + node.content_box().max.x;
            let mut pending = vec![header];
            while let Some(entity) = pending.pop() {
                if entity == ellipsis {
                    continue;
                }
                if entity != header && world.get::<bevy::ui::OverrideClip>(entity).is_some() {
                    continue;
                }
                if world.get::<Text>(entity).is_some()
                    && world.get::<bevy::text::EditableText>(entity).is_none()
                {
                    texts.push(entity);
                }
                if let (Some(layout), Some(node), Some(transform)) = (
                    world.get::<TextLayoutInfo>(entity),
                    world.get::<ComputedNode>(entity),
                    world.get::<UiGlobalTransform>(entity),
                ) && layout.size.x > 0.0
                    && transform.translation.x + node.content_box().min.x + layout.size.x
                        > right + 0.5
                {
                    overflow = true;
                    font = world.get::<TextFont>(entity).cloned();
                }
                if let Some(children) = world.get::<Children>(entity) {
                    pending.extend(children.iter());
                }
            }
        }
        if let Some(mut node) = world.get_mut::<Node>(ellipsis) {
            let display = if overflow {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
        if let Some(font) = font
            && let Some(mut output) = world.get_mut::<TextFont>(ellipsis)
        {
            output.set_if_neq(font);
        }
        position_ellipsis(world, header, ellipsis);
        for text in texts {
            project_title(world, header, text, overflow);
        }
    }
}

fn position_ellipsis(world: &mut World, header: Entity, ellipsis: Entity) {
    let Some(parent) = world.get::<ChildOf>(header).map(ChildOf::parent) else {
        return;
    };
    let (Some(node), Some(transform), Some(parent_node), Some(parent_transform)) = (
        world.get::<ComputedNode>(header),
        world.get::<UiGlobalTransform>(header),
        world.get::<ComputedNode>(parent),
        world.get::<UiGlobalTransform>(parent),
    ) else {
        return;
    };
    let Some(inverse) = parent_transform.try_inverse() else {
        return;
    };
    let position = inverse.transform_point2(transform.translation) + parent_node.size() * 0.5
        - parent_node.border.min_inset;
    let scale = node.inverse_scale_factor;
    let left = (position.x + node.size().x * 0.5 - node.border.max_inset.x) * scale - 34.0;
    let top = (position.y - node.size().y * 0.5 + node.border.min_inset.y) * scale + 6.0;
    let height = (node.size().y - node.border.min_inset.y - node.border.max_inset.y) * scale - 12.0;
    if let Some(mut output) = world.get_mut::<Node>(ellipsis)
        && (output.left != px(left)
            || output.top != px(top)
            || output.height != px(height)
            || output.right != Val::Auto
            || output.bottom != Val::Auto)
    {
        output.left = px(left);
        output.top = px(top);
        output.height = px(height);
        output.right = Val::Auto;
        output.bottom = Val::Auto;
    }
}

fn project_title(world: &mut World, header: Entity, text: Entity, elide: bool) {
    let Some(output) = world.entity(text).get_ref::<TextLayoutInfo>() else {
        return;
    };
    let source = match world.get::<ElidedLayout>(text) {
        Some(cached) if cached.projected_at == output.last_changed() => cached.source.clone(),
        _ => (*output).clone(),
    };
    if !elide {
        if world.get::<ElidedLayout>(text).is_some() {
            world
                .entity_mut(text)
                .insert(source)
                .remove::<ElidedLayout>();
        }
        return;
    }
    let (Some(header_node), Some(header_transform), Some(text_node), Some(text_transform)) = (
        world.get::<ComputedNode>(header),
        world.get::<UiGlobalTransform>(header),
        world.get::<ComputedNode>(text),
        world.get::<UiGlobalTransform>(text),
    ) else {
        return;
    };
    let Some(inverse) = text_transform.try_inverse() else {
        return;
    };
    let mut clip = header_node.content_box();
    clip.max.x = clip.max.x.min(
        header_node.size().x * 0.5
            - header_node.border.max_inset.x
            - 34.0 / header_node.inverse_scale_factor,
    );
    let mapping = inverse * header_transform.affine();
    let origin = text_node.content_box().min;
    let clip = Rect::from_corners(
        mapping.transform_point2(clip.min) - origin,
        mapping.transform_point2(clip.max) - origin,
    );
    let mut projected = source.clone();
    // Keep whole glyphs before the ellipsis, including when Text is on the Header itself.
    projected.glyphs.retain(|glyph| {
        let bounds = Rect::from_center_size(glyph.position, glyph.atlas_info.rect.size());
        glyph.line_index == 0 && bounds.max.x <= clip.max.x && bounds.max.x >= clip.min.x
    });
    for run in &mut projected.run_geometry {
        run.bounds = run.bounds.intersect(clip);
    }
    projected.run_geometry.retain(|run| !run.bounds.is_empty());
    projected
        .inline_boxes
        .retain(|(_, _, bounds)| clip.contains(bounds.min) && clip.contains(bounds.max));
    world.entity_mut(text).insert(projected);
    if let Some(layout) = world.entity(text).get_ref::<TextLayoutInfo>() {
        let projected_at = layout.last_changed();
        world.entity_mut(text).insert(ElidedLayout {
            source,
            projected_at,
        });
    }
}
