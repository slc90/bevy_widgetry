use crate::error::{CanvasResult as Result, PublicResult, busy, invalid, missing};
use crate::order::{CanvasItemOrder, container_items, container_items_mut};
use crate::{
    CanvasContainer, CanvasDocument, CanvasGeometry, CanvasGroup, CanvasIssue, CanvasLayerStack,
    CanvasLayersChanged, CanvasPaint, CanvasPart,
};
use bevy::{
    ecs::{
        entity::{EntityHashMap, EntityHashSet},
        lifecycle::HookContext,
        world::{DeferredWorld, WorldId},
    },
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_widgetry_diagnostics::ErrorFrame;
use std::collections::VecDeque;

const DEFAULT_CAPACITY: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasEditToken {
    world: WorldId,
    document: Entity,
    id: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum CanvasEditTarget {
    Document(Entity),
    Transaction(CanvasEditToken),
}

impl CanvasEditTarget {
    pub(crate) fn document(self) -> Entity {
        match self {
            Self::Document(document) => document,
            Self::Transaction(token) => token.document,
        }
    }
}

impl From<Entity> for CanvasEditTarget {
    fn from(document: Entity) -> Self {
        Self::Document(document)
    }
}

impl From<CanvasEditToken> for CanvasEditTarget {
    fn from(token: CanvasEditToken) -> Self {
        Self::Transaction(token)
    }
}

#[derive(Component)]
#[component(on_remove = despawn_retained_items)]
pub struct CanvasHistory {
    capacity: usize,
    next_id: u64,
    active: Option<(u64, Edit)>,
    replaying: bool,
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    retained: EntityHashMap<usize>,
    tombstones: EntityHashSet,
}

impl Default for CanvasHistory {
    fn default() -> Self {
        Self {
            capacity: DEFAULT_CAPACITY,
            next_id: 0,
            active: None,
            replaying: false,
            undo: VecDeque::new(),
            redo: Vec::new(),
            retained: EntityHashMap::default(),
            tombstones: EntityHashSet::default(),
        }
    }
}

#[derive(Component)]
#[component(on_remove = forget_tombstone)]
struct CanvasTombstone(Entity);

fn despawn_retained_items(mut world: DeferredWorld, context: HookContext) {
    if let Some(history) = world.get::<CanvasHistory>(context.entity) {
        let items = history.tombstones.iter().copied().collect::<Vec<_>>();
        for item in items {
            world.commands().entity(item).try_despawn();
        }
    }
}

fn forget_tombstone(mut world: DeferredWorld, context: HookContext) {
    if let Some(document) = world
        .get::<CanvasTombstone>(context.entity)
        .map(|item| item.0)
        && let Some(mut history) = world.get_mut::<CanvasHistory>(document)
    {
        history.tombstones.remove(&context.entity);
    }
}

#[derive(Default)]
struct Edit {
    transforms: EntityHashMap<Transform>,
    text: EntityHashMap<crate::CanvasText>,
    connectors: EntityHashMap<crate::CanvasConnector>,
    geometry: EntityHashMap<CanvasGeometry>,
    paint: EntityHashMap<CanvasPaint>,
    parents: EntityHashMap<Option<Entity>>,
    pending_detach: EntityHashSet,
    orders: HashMap<CanvasContainer, OrderBefore>,
    layers: Option<LayerLayout>,
    layer_move: bool,
}

#[derive(Default)]
struct Entry {
    transforms: Vec<Change<Transform>>,
    text: Vec<Change<crate::CanvasText>>,
    connectors: Vec<Change<crate::CanvasConnector>>,
    geometry: Vec<Change<CanvasGeometry>>,
    paint: Vec<Change<CanvasPaint>>,
    parents: Vec<Change<Option<Entity>>>,
    orders: Vec<OrderChange>,
    layers: Option<(LayerLayout, LayerLayout)>,
    layers_changed: bool,
    references: EntityHashSet,
}

struct Change<T> {
    entity: Entity,
    before: T,
    after: T,
}

struct OrderBefore {
    start: usize,
    tail: Vec<Entity>,
}

struct OrderChange {
    container: CanvasContainer,
    before: Vec<Entity>,
    after: Vec<Entity>,
    following: Vec<Entity>,
}

#[derive(PartialEq)]
struct LayerLayout {
    layers: Vec<crate::CanvasLayer>,
    default: crate::CanvasLayerId,
}

impl LayerLayout {
    fn capture(stack: &CanvasLayerStack) -> Self {
        Self {
            layers: stack
                .layers
                .iter()
                .map(|layer| crate::CanvasLayer {
                    id: layer.id,
                    name: layer.name.clone(),
                    visible: layer.visible,
                    locked: layer.locked,
                    items: Vec::new(),
                })
                .collect(),
            default: stack.default_layer,
        }
    }

    fn restore(&self, stack: &mut CanvasLayerStack) {
        let mut old_orders = std::mem::take(&mut stack.layers)
            .into_iter()
            .map(|layer| (layer.id, layer.items))
            .collect::<HashMap<_, _>>();
        stack.layers = self
            .layers
            .iter()
            .cloned()
            .map(|mut layer| {
                layer.items = old_orders.remove(&layer.id).unwrap_or_default();
                layer
            })
            .collect();
        stack.default_layer = self.default;
    }

    fn visible_layers(&self) -> impl Iterator<Item = crate::CanvasLayerId> + '_ {
        self.layers
            .iter()
            .filter(|layer| layer.visible())
            .map(crate::CanvasLayer::id)
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Undo,
    Redo,
}

impl Direction {
    fn value<'a, T>(self, before: &'a T, after: &'a T) -> &'a T {
        match self {
            Self::Undo => before,
            Self::Redo => after,
        }
    }

    fn pair<T>(self, before: T, after: T) -> (T, T) {
        match self {
            Self::Undo => (after, before),
            Self::Redo => (before, after),
        }
    }
}

impl CanvasDocument {
    pub fn edit<T>(
        world: &mut World,
        document: Entity,
        edit: impl FnOnce(&mut World, CanvasEditToken) -> PublicResult<T>,
    ) -> PublicResult<T> {
        let frame = || ErrorFrame::new("canvas.CanvasDocument.edit");
        let token = begin(world, document).map_err(|error| error.into_failure(frame()))?;
        let result = edit(world, token);
        commit(world, token).map_err(|error| error.into_failure(frame()))?;
        result
    }

    pub fn can_undo(world: &World, document: Entity) -> PublicResult<bool> {
        history(world, document)
            .map(|history| !history.busy() && !history.undo.is_empty())
            .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasDocument.can_undo")))
    }

    pub fn can_redo(world: &World, document: Entity) -> PublicResult<bool> {
        history(world, document)
            .map(|history| !history.busy() && !history.redo.is_empty())
            .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasDocument.can_redo")))
    }

    pub fn undo(world: &mut World, document: Entity) -> PublicResult<bool> {
        replay(world, document, Direction::Undo)
            .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasDocument.undo")))
    }

    pub fn redo(world: &mut World, document: Entity) -> PublicResult<bool> {
        replay(world, document, Direction::Redo)
            .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasDocument.redo")))
    }

    pub fn set_history_capacity(
        world: &mut World,
        document: Entity,
        capacity: usize,
    ) -> PublicResult {
        (|| -> Result {
            ensure_idle(world, document)?;
            let mut history = history_mut(world, document)?;
            history.capacity = capacity;
            let released = history.trim();
            collect_tombstones(world, document, released);
            Ok(())
        })()
        .map_err(|error| {
            error.into_failure(ErrorFrame::new(
                "canvas.CanvasDocument.set_history_capacity",
            ))
        })
    }

    pub fn clear_history(world: &mut World, document: Entity) -> PublicResult {
        (|| -> Result {
            ensure_idle(world, document)?;
            let mut history = history_mut(world, document)?;
            let released = history.retained.keys().copied().collect::<Vec<_>>();
            history.undo.clear();
            history.redo.clear();
            history.retained.clear();
            collect_tombstones(world, document, released);
            Ok(())
        })()
        .map_err(|error| error.into_failure(ErrorFrame::new("canvas.CanvasDocument.clear_history")))
    }
}

impl CanvasHistory {
    fn busy(&self) -> bool {
        self.replaying || self.active.is_some()
    }

    fn release(&mut self, entry: Entry, released: &mut Vec<Entity>) {
        for entity in entry.references {
            if let Some(count) = self.retained.get_mut(&entity) {
                *count -= 1;
                if *count == 0 {
                    self.retained.remove(&entity);
                    released.push(entity);
                }
            }
        }
    }

    fn trim(&mut self) -> Vec<Entity> {
        let mut released = Vec::new();
        while self.undo.len() + self.redo.len() > self.capacity {
            let Some(entry) = self
                .undo
                .pop_front()
                .or_else(|| (!self.redo.is_empty()).then(|| self.redo.remove(0)))
            else {
                break;
            };
            self.release(entry, &mut released);
        }
        released
    }
}

fn history(world: &World, document: Entity) -> Result<&CanvasHistory> {
    crate::model::validate_document(world, document)?;
    world
        .get::<CanvasHistory>(document)
        .ok_or_else(|| missing(CanvasPart::History).at("document", document))
}

fn history_mut(world: &mut World, document: Entity) -> Result<Mut<'_, CanvasHistory>> {
    world
        .get_mut::<CanvasHistory>(document)
        .ok_or_else(|| missing(CanvasPart::History).at("document", document))
}

fn ensure_idle(world: &World, document: Entity) -> Result {
    if history(world, document)?.busy() {
        return Err(busy().at("document", document));
    }
    Ok(())
}

pub(crate) fn is_busy(world: &World, document: Entity) -> bool {
    world
        .get::<CanvasHistory>(document)
        .is_some_and(CanvasHistory::busy)
}

fn begin(world: &mut World, document: Entity) -> Result<CanvasEditToken> {
    ensure_idle(world, document)?;
    let mut history = history_mut(world, document)?;
    let id = history.next_id;
    history.next_id = id
        .checked_add(1)
        .ok_or_else(|| invalid(CanvasIssue::EditTokenExhausted))?;
    history.active = Some((id, Edit::default()));
    Ok(CanvasEditToken {
        world: world.id(),
        document,
        id,
    })
}

pub(crate) fn begin_gesture(
    world: &mut World,
    document: Entity,
) -> Result<Option<CanvasEditToken>> {
    if is_busy(world, document) {
        return Ok(None);
    }
    begin(world, document).map(Some)
}

fn verify(world: &World, token: CanvasEditToken) -> Result {
    if token.world == world.id()
        && history(world, token.document)?
            .active
            .as_ref()
            .is_some_and(|(id, _)| *id == token.id)
    {
        return Ok(());
    }
    Err(invalid(CanvasIssue::EditToken).at("document", token.document))
}

pub(crate) fn apply<T>(
    world: &mut World,
    target: impl Into<CanvasEditTarget>,
    operation: impl FnOnce(&mut World, Entity) -> Result<T>,
) -> Result<T> {
    match target.into() {
        CanvasEditTarget::Transaction(token) => {
            verify(world, token)?;
            let result = operation(world, token.document);
            detach_pending(world, token.document)?;
            // Bevy defers the inverse relationship update for batched inserts.
            // Settle it before another operation re-parents the same item.
            world.flush();
            result
        }
        CanvasEditTarget::Document(document) => {
            let token = begin(world, document)?;
            let result = operation(world, document);
            detach_pending(world, document)?;
            commit(world, token)?;
            result
        }
    }
}

fn active_edit(world: &mut World, document: Entity) -> Result<Mut<'_, Edit>> {
    history_mut(world, document)?
        .filter_map_unchanged(|history| history.active.as_mut().map(|(_, edit)| edit))
        .ok_or_else(|| missing(CanvasPart::History).at("document", document))
}

pub(crate) fn set_transform(
    world: &mut World,
    document: Entity,
    item: Entity,
    value: Transform,
) -> Result {
    if record_transform(world, document, item, &value)? {
        world.insert_batch([(item, value)]);
    }
    Ok(())
}

fn record_transform(
    world: &mut World,
    document: Entity,
    item: Entity,
    value: &Transform,
) -> Result<bool> {
    record_component(
        world,
        document,
        item,
        value,
        |edit| &mut edit.transforms,
        |edit| &edit.transforms,
    )
}

pub(crate) fn set_text(
    world: &mut World,
    document: Entity,
    item: Entity,
    text: crate::CanvasText,
) -> Result {
    if record_component(
        world,
        document,
        item,
        &text,
        |edit| &mut edit.text,
        |edit| &edit.text,
    )? {
        world.insert_batch([(item, text)]);
    }
    Ok(())
}

pub(crate) fn set_connector(
    world: &mut World,
    document: Entity,
    item: Entity,
    connector: crate::CanvasConnector,
) -> Result {
    if record_component(
        world,
        document,
        item,
        &connector,
        |edit| &mut edit.connectors,
        |edit| &edit.connectors,
    )? {
        world.insert_batch([(item, connector)]);
    }
    Ok(())
}

pub(crate) fn set_geometry(
    world: &mut World,
    document: Entity,
    item: Entity,
    value: CanvasGeometry,
) -> Result {
    if record_component(
        world,
        document,
        item,
        &value,
        |edit| &mut edit.geometry,
        |edit| &edit.geometry,
    )? {
        world.insert_batch([(item, value)]);
    }
    Ok(())
}

pub(crate) fn set_shape(
    world: &mut World,
    document: Entity,
    item: Entity,
    geometry: CanvasGeometry,
    transform: Transform,
    paint: CanvasPaint,
) -> Result {
    let geometry_changed = record_component(
        world,
        document,
        item,
        &geometry,
        |edit| &mut edit.geometry,
        |edit| &edit.geometry,
    )?;
    let transform_changed = record_transform(world, document, item, &transform)?;
    let paint_changed = record_component(
        world,
        document,
        item,
        &paint,
        |edit| &mut edit.paint,
        |edit| &edit.paint,
    )?;
    if geometry_changed || transform_changed || paint_changed {
        world.insert_batch([(item, (geometry, transform, paint))]);
    }
    Ok(())
}

fn record_component<T: Component + Clone + PartialEq>(
    world: &mut World,
    document: Entity,
    item: Entity,
    value: &T,
    field: impl Fn(&mut Edit) -> &mut EntityHashMap<T>,
    recorded: impl Fn(&Edit) -> &EntityHashMap<T>,
) -> Result<bool> {
    let component = world
        .get::<T>(item)
        .ok_or_else(|| missing(CanvasPart::History).at("item", item))?;
    if component == value {
        return Ok(false);
    }
    if world
        .get::<CanvasHistory>(document)
        .and_then(|history| history.active.as_ref())
        .is_some_and(|(_, edit)| recorded(edit).contains_key(&item))
    {
        return Ok(true);
    }
    let before = component.clone();
    let mut edit = active_edit(world, document)?;
    field(&mut edit).insert(item, before);
    Ok(true)
}

pub(crate) fn record_created(world: &mut World, document: Entity, item: Entity) -> Result {
    active_edit(world, document)?.parents.insert(item, None);
    Ok(())
}

fn record_parent(world: &mut World, document: Entity, item: Entity) -> Result<Mut<'_, Edit>> {
    let before = world.get::<ChildOf>(item).map(ChildOf::parent);
    let mut edit = active_edit(world, document)?;
    edit.parents.entry(item).or_insert(before);
    Ok(edit)
}

pub(crate) fn detach_item(world: &mut World, document: Entity, item: Entity) -> Result {
    record_parent(world, document, item)?
        .pending_detach
        .insert(item);
    Ok(())
}

pub(crate) fn reparent_items(
    world: &mut World,
    document: Entity,
    parent: Entity,
    updates: Vec<(Entity, Transform)>,
) -> Result {
    for (item, transform) in &updates {
        record_transform(world, document, *item, transform)?;
        record_parent(world, document, *item)?
            .pending_detach
            .remove(item);
    }
    world.insert_batch(
        updates
            .into_iter()
            .map(|(item, transform)| (item, (transform, ChildOf(parent)))),
    );
    Ok(())
}

fn detach_pending(world: &mut World, document: Entity) -> Result {
    let mut edit = active_edit(world, document)?;
    if edit.pending_detach.is_empty() {
        return Ok(());
    }
    let items = std::mem::take(&mut edit.pending_detach);
    retain_items(world, document, items.iter().copied())?;
    for &item in &items {
        if let Ok(mut item) = world.get_entity_mut(item) {
            item.remove::<ChildOf>();
        }
    }
    Ok(())
}

fn retain_items(
    world: &mut World,
    document: Entity,
    items: impl Iterator<Item = Entity> + Clone,
) -> Result {
    // ChildOf removal flushes queued commands. The document must own its
    // detached roots even if one of those commands despawns the document.
    history_mut(world, document)?
        .tombstones
        .extend(items.clone());
    world.insert_batch(items.map(|item| (item, CanvasTombstone(document))));
    Ok(())
}

pub(crate) fn edit_order(
    world: &mut World,
    document: Entity,
    container: CanvasContainer,
    start: usize,
    update: impl FnOnce(&mut Vec<Entity>),
) -> Result {
    capture_order(world, document, container, start)?;
    update(container_items_mut(world, document, container)?);
    Ok(())
}

pub(crate) fn capture_order(
    world: &mut World,
    document: Entity,
    container: CanvasContainer,
    start: usize,
) -> Result {
    let order = container_items(world, document, container)?;
    let prior = history(world, document)?
        .active
        .as_ref()
        .and_then(|(_, edit)| edit.orders.get(&container));
    let end = prior.map_or(order.len(), |before| before.start);
    if prior.is_some() && start >= end {
        return Ok(());
    }
    let mut tail = order
        .get(start..end)
        .ok_or_else(|| missing(CanvasPart::DocumentOrder))?
        .to_vec();
    let mut edit = active_edit(world, document)?;
    if let Some(prior) = edit.orders.get_mut(&container) {
        tail.append(&mut prior.tail);
    }
    edit.orders.insert(container, OrderBefore { start, tail });
    Ok(())
}

pub(crate) fn repair_order(
    world: &mut World,
    document: Entity,
    container: CanvasContainer,
    removed: &HashSet<Entity>,
) -> Result {
    let start = world
        .get::<CanvasHistory>(document)
        .and_then(|history| history.active.as_ref())
        .and_then(|(_, edit)| edit.orders.get(&container))
        .map(|before| before.start);
    if let Some(start) = start {
        let removed_prefix = raw_order(world, document, container)
            .iter()
            .take(start)
            .filter(|item| removed.contains(*item))
            .count();
        if let Some(before) = active_edit(world, document)?.orders.get_mut(&container) {
            // Lazy model repair is outside the edit. Normalize both sides of
            // the pending patch when it removes items from the saved prefix.
            before.start -= removed_prefix;
            before.tail.retain(|item| !removed.contains(item));
        }
    }
    container_items_mut(world, document, container)?.retain(|item| !removed.contains(item));
    Ok(())
}

pub(crate) fn capture_layers(world: &mut World, document: Entity) -> Result {
    if history(world, document)?
        .active
        .as_ref()
        .is_some_and(|(_, edit)| edit.layers.is_some())
    {
        return Ok(());
    }
    let before = LayerLayout::capture(crate::order::stack(world, document)?);
    active_edit(world, document)?.layers = Some(before);
    Ok(())
}

pub(crate) fn record_layer_move(world: &mut World, document: Entity) -> Result {
    active_edit(world, document)?.layer_move = true;
    Ok(())
}

fn raw_order(world: &World, document: Entity, container: CanvasContainer) -> &[Entity] {
    match container {
        CanvasContainer::Layer(layer) => world
            .get::<CanvasLayerStack>(document)
            .and_then(|stack| stack.layer(layer))
            .map(|entry| entry.items.as_slice()),
        CanvasContainer::Group(group) => world
            .get::<CanvasItemOrder>(group)
            .map(|order| order.0.as_slice()),
    }
    .unwrap_or_default()
}

impl OrderChange {
    fn from_edit(
        world: &World,
        document: Entity,
        container: CanvasContainer,
        mut before: OrderBefore,
    ) -> Option<Self> {
        let after = raw_order(world, document, container).get(before.start..)?;
        if before.tail == after {
            return None;
        }
        let prefix = before
            .tail
            .iter()
            .zip(after)
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before.tail[prefix..]
            .iter()
            .rev()
            .zip(after[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        before.tail.truncate(before.tail.len() - suffix);
        before.tail.drain(..prefix);
        Some(Self {
            container,
            before: before.tail,
            after: after[prefix..after.len() - suffix].to_vec(),
            following: after[after.len() - suffix..].to_vec(),
        })
    }

    fn replay(&self, world: &World, order: &mut Vec<Entity>, direction: Direction) -> Result {
        let (expected, replacement) = direction.pair(&self.before, &self.after);
        let alive = |entity: &Entity| world.get_entity(*entity).is_ok();
        order.retain(alive);
        // A shared member missing from the current order was removed by
        // model repair. It should stay absent on either side of the splice.
        let current = order.iter().copied().collect::<EntityHashSet>();
        let replacement_items = replacement.iter().copied().collect::<EntityHashSet>();
        let missing_neighbors = expected
            .iter()
            .copied()
            .filter(|item| !current.contains(item) && replacement_items.contains(item))
            .collect::<EntityHashSet>();
        let available = |item: &Entity| alive(item) && !missing_neighbors.contains(item);
        let end = if self.following.is_empty() {
            order.len()
        } else {
            let following = self.following.iter().copied().collect::<EntityHashSet>();
            order
                .iter()
                .position(|item| following.contains(item))
                .unwrap_or(order.len())
        };
        let expected = expected.iter().filter(|item| available(item));
        let start = end
            .checked_sub(expected.clone().count())
            .ok_or_else(|| missing(CanvasPart::History))?;
        if !order[start..end].iter().eq(expected) {
            return Err(missing(CanvasPart::History));
        }
        drop(order.splice(
            start..end,
            replacement.iter().filter(|item| available(item)).copied(),
        ));
        Ok(())
    }
}

fn changes<T: PartialEq>(
    before: EntityHashMap<T>,
    mut read: impl FnMut(Entity) -> Option<T>,
) -> Vec<Change<T>> {
    before
        .into_iter()
        .filter_map(|(entity, before)| {
            let after = read(entity)?;
            (after != before).then_some(Change {
                entity,
                before,
                after,
            })
        })
        .collect()
}

impl Edit {
    fn transient_items(&self, world: &World) -> EntityHashSet {
        let mut transient = EntityHashSet::default();
        for (&root, before) in &self.parents {
            if before.is_some() || world.get::<ChildOf>(root).is_some() {
                continue;
            }
            transient.insert(root);
            let mut pending = vec![root];
            let mut subtree = EntityHashSet::default();
            let mut entirely_new = true;
            while let Some(item) = pending.pop() {
                if self.parents.get(&item) != Some(&None) {
                    // A pre-existing member needs this root to remain available to redo.
                    entirely_new = false;
                    break;
                }
                if subtree.insert(item)
                    && let Some(children) = world.get::<Children>(item)
                {
                    pending.extend(children.iter());
                }
            }
            if entirely_new {
                transient.extend(subtree);
            }
        }
        transient
    }

    fn finish(self, world: &World, document: Entity) -> (Entry, Vec<Entity>) {
        let candidates = self.parents.keys().copied().collect();
        let transient = self.transient_items(world);
        let orders = self.orders.into_iter()
            .filter(|(container, _)| !matches!(container, CanvasContainer::Group(group) if transient.contains(group)))
            .filter_map(|(container, before)| OrderChange::from_edit(world, document, container, before))
            .collect::<Vec<_>>();
        let layers = self.layers.and_then(|before| {
            let after = LayerLayout::capture(world.get::<CanvasLayerStack>(document)?);
            (before != after).then_some((before, after))
        });
        let layers_changed = layers.is_some()
            || (self.layer_move
                && orders
                    .iter()
                    .any(|order| matches!(order.container, CanvasContainer::Layer(_))));
        let mut entry = Entry {
            transforms: changes(self.transforms, |entity| {
                (!transient.contains(&entity))
                    .then(|| world.get::<Transform>(entity).copied())
                    .flatten()
            }),
            text: changes(self.text, |entity| {
                (!transient.contains(&entity))
                    .then(|| world.get::<crate::CanvasText>(entity).cloned())
                    .flatten()
            }),
            connectors: changes(self.connectors, |entity| {
                (!transient.contains(&entity))
                    .then(|| world.get::<crate::CanvasConnector>(entity).cloned())
                    .flatten()
            }),
            geometry: changes(self.geometry, |entity| {
                (!transient.contains(&entity))
                    .then(|| world.get::<CanvasGeometry>(entity).cloned())
                    .flatten()
            }),
            paint: changes(self.paint, |entity| {
                (!transient.contains(&entity))
                    .then(|| world.get::<CanvasPaint>(entity).copied())
                    .flatten()
            }),
            parents: changes(self.parents, |entity| {
                (!transient.contains(&entity)).then_some(())?;
                world
                    .get_entity(entity)
                    .ok()
                    .map(|item| item.get::<ChildOf>().map(ChildOf::parent))
            }),
            orders,
            layers,
            layers_changed,
            references: EntityHashSet::default(),
        };
        entry.record_references(document);
        (entry, candidates)
    }
}

impl Entry {
    fn record_references(&mut self, document: Entity) {
        self.references
            .extend(self.transforms.iter().map(|change| change.entity));
        self.references
            .extend(self.text.iter().map(|change| change.entity));
        self.references
            .extend(self.connectors.iter().map(|change| change.entity));
        self.references
            .extend(self.geometry.iter().map(|change| change.entity));
        self.references
            .extend(self.paint.iter().map(|change| change.entity));
        for parent in &self.parents {
            self.references.insert(parent.entity);
            self.references.extend(parent.before);
            self.references.extend(parent.after);
        }
        // Orders only refer to attached items. The edit that detaches an item
        // outlives those earlier orders when the history is trimmed.
        self.references.remove(&document);
    }

    fn is_empty(&self) -> bool {
        self.transforms.is_empty()
            && self.text.is_empty()
            && self.connectors.is_empty()
            && self.geometry.is_empty()
            && self.paint.is_empty()
            && self.parents.is_empty()
            && self.orders.is_empty()
            && self.layers.is_none()
    }
}

fn close(world: &mut World, token: CanvasEditToken) -> Option<(bool, bool)> {
    if token.world != world.id() {
        return None;
    }
    let document = token.document;
    let mut history = world.get_mut::<CanvasHistory>(document)?;
    if history
        .active
        .as_ref()
        .is_none_or(|(id, _)| *id != token.id)
    {
        return None;
    }
    let (_, edit) = history.active.take()?;
    let (entry, mut released) = edit.finish(world, document);
    let changed = !entry.is_empty();
    let layers_changed = entry.layers_changed;
    let mut history = world.get_mut::<CanvasHistory>(document)?;
    if changed {
        for &entity in &entry.references {
            *history.retained.entry(entity).or_default() += 1;
        }
        history.undo.push_back(entry);
        while let Some(entry) = history.redo.pop() {
            history.release(entry, &mut released);
        }
        released.extend(history.trim());
    }
    collect_tombstones(world, document, released);
    Some((changed, layers_changed))
}

fn collect_tombstones(world: &mut World, document: Entity, released: Vec<Entity>) {
    for entity in released {
        if world
            .get::<CanvasTombstone>(entity)
            .is_some_and(|owner| owner.0 == document)
            && world
                .get::<CanvasHistory>(document)
                .is_some_and(|history| !history.retained.contains_key(&entity))
        {
            world.despawn(entity);
        }
    }
}

fn commit(world: &mut World, token: CanvasEditToken) -> Result<bool> {
    let (changed, layers_changed) = close(world, token)
        .ok_or_else(|| invalid(CanvasIssue::EditToken).at("document", token.document))?;
    if layers_changed {
        notify_layers(world, token.document)?;
    }
    world.flush();
    Ok(changed)
}

pub(crate) fn finish_gesture(world: &mut World, token: CanvasEditToken) {
    let _closed = close(world, token);
}

fn notify_layers(world: &mut World, document: Entity) -> Result {
    crate::view::repair_layers(world, document)?;
    if world.get::<CanvasDocument>(document).is_some() {
        world.trigger(CanvasLayersChanged { entity: document });
    }
    Ok(())
}

#[derive(Default)]
struct StructureReplay {
    layers: Option<CanvasLayerStack>,
    groups: EntityHashMap<Vec<Entity>>,
}

impl StructureReplay {
    fn build(world: &World, document: Entity, entry: &Entry, direction: Direction) -> Result<Self> {
        let mut replay = Self::default();
        let mut layer_orders = HashMap::<_, _>::default();
        for change in &entry.orders {
            let mut order = raw_order(world, document, change.container).to_vec();
            change.replay(world, &mut order, direction)?;
            match change.container {
                CanvasContainer::Layer(layer) => {
                    layer_orders.insert(layer, order);
                }
                CanvasContainer::Group(group) => {
                    if world.get::<CanvasItemOrder>(group).is_none() {
                        return Err(missing(CanvasPart::History).at("group", group));
                    }
                    replay.groups.insert(group, order);
                }
            }
        }
        if entry.layers.is_some() || !layer_orders.is_empty() {
            let mut stack = crate::order::stack(world, document)?.clone();
            if let Some((before, after)) = &entry.layers {
                direction.value(before, after).restore(&mut stack);
            }
            for layer in &mut stack.layers {
                if let Some(order) = layer_orders.remove(&layer.id) {
                    layer.items = order;
                }
            }
            if layer_orders.values().any(|order| !order.is_empty()) {
                return Err(missing(CanvasPart::History));
            }
            replay.layers = Some(stack);
        }
        Ok(replay)
    }

    fn write(self, world: &mut World, document: Entity) {
        if let Some(stack) = self.layers
            && let Some(mut current) = world.get_mut::<CanvasLayerStack>(document)
        {
            *current = stack;
        }
        for (group, order) in self.groups {
            if let Some(mut current) = world.get_mut::<CanvasItemOrder>(group) {
                current.0 = order;
            }
        }
    }
}

fn replay(world: &mut World, document: Entity, direction: Direction) -> Result<bool> {
    ensure_idle(world, document)?;
    let mut history = history_mut(world, document)?;
    let Some(entry) = (match direction {
        Direction::Undo => history.undo.pop_back(),
        Direction::Redo => history.redo.pop(),
    }) else {
        return Ok(false);
    };
    let newly_visible = entry
        .layers
        .as_ref()
        .map(|(before, after)| {
            let (current, restored) = direction.pair(before, after);
            let visible = current.visible_layers().collect::<HashSet<_>>();
            restored
                .visible_layers()
                .filter(|layer| !visible.contains(layer))
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    history.replaying = true;
    let result = restore(world, document, &entry, direction, &newly_visible);
    let layers_changed = entry.layers_changed;
    let mut history = history_mut(world, document)?;
    history.replaying = false;
    match (result.is_ok(), direction) {
        (true, Direction::Undo) | (false, Direction::Redo) => history.redo.push(entry),
        (true, Direction::Redo) | (false, Direction::Undo) => history.undo.push_back(entry),
    }
    result.map_err(|error| error.at("document", document))?;
    world.flush();
    repair_revealed_layers(world, document, &newly_visible)?;
    if layers_changed {
        notify_layers(world, document)?;
    } else {
        crate::view::repair_layers(world, document)?;
    }
    Ok(true)
}

fn repair_revealed_layers(
    world: &mut World,
    document: Entity,
    newly_visible: &HashSet<crate::CanvasLayerId>,
) -> Result {
    if !newly_visible.is_empty() {
        drop(CanvasDocument::shape_order_in(world, document, |layer| {
            layer.visible() && newly_visible.contains(&layer.id())
        })?);
    }
    Ok(())
}

fn validate_replay_owner(world: &World, document: Entity, entity: Entity) -> Result {
    if world.get_entity(entity).is_err() {
        return Err(missing(CanvasPart::History).at("item", entity));
    }
    let mut current = entity;
    let mut ancestors = EntityHashSet::default();
    loop {
        if current == document {
            return Ok(());
        }
        if world.get::<CanvasDocument>(current).is_some() {
            break;
        }
        let Some(parent) = world.get::<ChildOf>(current).map(ChildOf::parent) else {
            if world
                .get::<CanvasTombstone>(current)
                .is_some_and(|owner| owner.0 == document)
            {
                return Ok(());
            }
            break;
        };
        current = parent;
        if current != document && !ancestors.insert(current) {
            break;
        }
    }
    Err(crate::model::invalid_ownership(world, entity).at("item", entity))
}

fn restore(
    world: &mut World,
    document: Entity,
    entry: &Entry,
    direction: Direction,
    newly_visible: &HashSet<crate::CanvasLayerId>,
) -> Result {
    let items = entry
        .transforms
        .iter()
        .map(|change| change.entity)
        .chain(entry.text.iter().map(|change| change.entity))
        .chain(entry.connectors.iter().map(|change| change.entity))
        .chain(entry.geometry.iter().map(|change| change.entity))
        .chain(entry.paint.iter().map(|change| change.entity))
        .chain(entry.parents.iter().map(|change| change.entity))
        .chain(
            entry
                .parents
                .iter()
                .filter_map(|change| *direction.value(&change.before, &change.after)),
        )
        .chain(entry.orders.iter().filter_map(|change| {
            if let CanvasContainer::Group(group) = change.container {
                Some(group)
            } else {
                None
            }
        }));
    for entity in items {
        validate_replay_owner(world, document, entity)?;
    }
    for order in &entry.orders {
        let before = order.before.iter().copied().collect::<EntityHashSet>();
        let after = order.after.iter().copied().collect::<EntityHashSet>();
        for &entity in before.symmetric_difference(&after) {
            if world.get_entity(entity).is_ok() {
                validate_replay_owner(world, document, entity)?;
            }
        }
    }
    let parent = |change: &Change<Option<Entity>>| *direction.value(&change.before, &change.after);
    let attachments = ordered_attachments(world, document, &entry.parents, parent)?;
    let structure = StructureReplay::build(world, document, entry, direction)?;
    let (mut shapes, mut transforms) = prepare_components(world, entry, direction)?;
    let mut connectors = entry
        .connectors
        .iter()
        .map(|change| {
            (
                change.entity,
                direction.value(&change.before, &change.after).clone(),
            )
        })
        .collect::<EntityHashMap<_>>();

    structure.write(world, document);
    let mut group_transforms = Vec::new();
    if !attachments.is_empty() {
        // ChildOf observers can read the new group transform as members attach.
        // Publish its Insert event once the destination hierarchy is in place.
        transforms.retain(|&item, transform| {
            if world.get::<CanvasGroup>(item).is_some()
                && let Some(mut current) = world.get_mut::<Transform>(item)
            {
                *current = *transform;
                group_transforms.push((item, *transform));
                false
            } else {
                true
            }
        });
    }
    for (item, parent) in attachments {
        let parent = ChildOf(parent);
        if let Some(shape) = shapes.remove(&item) {
            world.insert_batch([(item, (shape, parent))]);
        } else if let Some(connector) = connectors.remove(&item) {
            if let Some(transform) = transforms.remove(&item) {
                world.insert_batch([(item, (connector, transform, parent))]);
            } else {
                world.insert_batch([(item, (connector, parent))]);
            }
        } else if let Some(transform) = transforms.remove(&item) {
            world.insert_batch([(item, (transform, parent))]);
        } else {
            world.insert_batch([(item, parent)]);
        }
    }
    world.insert_batch(entry.text.iter().map(|change| {
        (
            change.entity,
            direction.value(&change.before, &change.after).clone(),
        )
    }));
    for (item, connector) in connectors {
        if let Some(transform) = transforms.remove(&item) {
            world.insert_batch([(item, (connector, transform))]);
        } else {
            world.insert_batch([(item, connector)]);
        }
    }
    world.insert_batch(shapes);
    world.insert_batch(transforms);
    world.insert_batch(group_transforms);
    repair_revealed_layers(world, document, newly_visible)?;

    // Removing a component flushes queued observers. At this point the new
    // order and all live links are already visible to those observers.
    if !entry.parents.is_empty() {
        retain_items(
            world,
            document,
            entry
                .parents
                .iter()
                .filter(|change| parent(change).is_none())
                .map(|change| change.entity),
        )?;
    }
    for change in &entry.parents {
        if let Ok(mut item) = world.get_entity_mut(change.entity) {
            if parent(change).is_none() {
                item.remove::<ChildOf>();
            } else if item.contains::<CanvasTombstone>() {
                item.remove::<CanvasTombstone>();
            }
        }
    }
    Ok(())
}

fn ordered_attachments(
    world: &World,
    document: Entity,
    changes: &[Change<Option<Entity>>],
    parent: impl Fn(&Change<Option<Entity>>) -> Option<Entity>,
) -> Result<Vec<(Entity, Entity)>> {
    let parents = changes
        .iter()
        .map(|change| (change.entity, parent(change)))
        .collect::<EntityHashMap<_>>();
    let mut attachments = Vec::with_capacity(parents.len());
    let mut ancestors = EntityHashSet::default();
    for (&item, &parent) in &parents {
        let Some(parent) = parent else { continue };
        ancestors.clear();
        let mut current = parent;
        while current != document {
            if current == item || !ancestors.insert(current) {
                return Err(crate::model::invalid_ownership(world, item).at("item", item));
            }
            let next = parents
                .get(&current)
                .copied()
                .unwrap_or_else(|| world.get::<ChildOf>(current).map(ChildOf::parent));
            let Some(next) = next else { break };
            current = next;
        }
        attachments.push((ancestors.len(), item, parent));
    }
    // Insert observers must see the destination hierarchy as well as its order.
    attachments.sort_unstable_by_key(|&(depth, _, _)| depth);
    Ok(attachments
        .into_iter()
        .map(|(_, item, parent)| (item, parent))
        .collect())
}

fn prepare_components(
    world: &World,
    entry: &Entry,
    direction: Direction,
) -> Result<(
    EntityHashMap<(CanvasGeometry, Transform, CanvasPaint)>,
    EntityHashMap<Transform>,
)> {
    let mut transforms = entry
        .transforms
        .iter()
        .map(|change| {
            (
                change.entity,
                *direction.value(&change.before, &change.after),
            )
        })
        .collect::<EntityHashMap<_>>();
    let mut geometries = entry
        .geometry
        .iter()
        .map(|change| {
            (
                change.entity,
                direction.value(&change.before, &change.after).clone(),
            )
        })
        .collect::<EntityHashMap<_>>();
    let mut paints = entry
        .paint
        .iter()
        .map(|change| {
            (
                change.entity,
                *direction.value(&change.before, &change.after),
            )
        })
        .collect::<EntityHashMap<_>>();
    let shapes = geometries
        .keys()
        .chain(paints.keys())
        .copied()
        .collect::<EntityHashSet>();
    let mut updates = EntityHashMap::with_capacity(shapes.len());
    for shape in shapes {
        let missing_data = || missing(CanvasPart::History).at("shape", shape);
        let geometry = geometries
            .remove(&shape)
            .or_else(|| world.get::<CanvasGeometry>(shape).cloned())
            .ok_or_else(missing_data)?;
        let transform = transforms
            .remove(&shape)
            .or_else(|| world.get::<Transform>(shape).copied())
            .ok_or_else(missing_data)?;
        let paint = paints
            .remove(&shape)
            .or_else(|| world.get::<CanvasPaint>(shape).copied())
            .ok_or_else(missing_data)?;
        updates.insert(shape, (geometry, transform, paint));
    }
    Ok((updates, transforms))
}
