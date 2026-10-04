use crate::model::contract_error;
use crate::{
    WidgetryFileDialogAction, WidgetryFileDialogProps, WidgetryFileDialogResult,
    WidgetryFileDialogSessionId, WidgetryFileDialogState, WidgetryFileDialogStorage,
};
use bevy::prelude::*;
use bevy_widgetry_log::widgetry_info;

#[derive(SceneComponent, FromTemplate, Default)]
#[scene(WidgetryFileDialogProps)]
pub struct WidgetryFileDialog;

pub struct WidgetryFileDialogPlugin;

pub struct WidgetryFileDialogHeadlessPlugin;

#[derive(EntityEvent, Clone, Debug)]
pub struct WidgetryFileDialogResultEvent {
    pub entity: Entity,
    pub session: WidgetryFileDialogSessionId,
    pub result: WidgetryFileDialogResult,
}

#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct WidgetryFileDialogChangeEvent {
    pub entity: Entity,
    pub session: WidgetryFileDialogSessionId,
}

impl Plugin for WidgetryFileDialogPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryFileDialogHeadlessPlugin>() {
            app.add_plugins(WidgetryFileDialogHeadlessPlugin);
        }
        widgetry_info!("WidgetryFileDialogPlugin 注册完成");
    }
}

impl Plugin for WidgetryFileDialogHeadlessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WidgetryFileDialogStorage>();
        widgetry_info!("WidgetryFileDialogHeadlessPlugin 注册完成");
    }
}

impl WidgetryFileDialog {
    fn scene(props: WidgetryFileDialogProps) -> impl Scene {
        bsn! {
            template(move |context| {
                if let Some(state) = context.entity.get::<WidgetryFileDialogState>() { return Ok(state.clone()); }
                let mut state = WidgetryFileDialogState::new(props.clone())
                    .map_err(|error| bevy_widgetry_core::scene::logged_error(error.to_string()))?;
                if let Some(snapshot) = state.storage_scope().and_then(|scope|
                    context.entity.world().get_resource::<WidgetryFileDialogStorage>()
                        .and_then(|storage| storage.snapshot(scope))).cloned() {
                    state.initialize_storage(snapshot);
                }
                Ok(state)
            })
            WidgetryFileDialog
        }
    }

    pub fn apply(
        world: &mut World,
        root: Entity,
        action: WidgetryFileDialogAction,
    ) -> Result<bool, BevyError> {
        if world.get::<Self>(root).is_none() {
            return Err(contract_error("invalid FileDialog root"));
        }
        let mut next = world
            .get::<WidgetryFileDialogState>(root)
            .ok_or_else(|| contract_error("FileDialog state missing"))?
            .clone();
        let pin = match &action {
            WidgetryFileDialogAction::Pin(path) => Some((path.clone(), true)),
            WidgetryFileDialogAction::Unpin(path) => Some((path.clone(), false)),
            _ => None,
        };
        let reopen = action == WidgetryFileDialogAction::Reopen;
        let store_pin_changes = pin.as_ref().is_some_and(|(path, add)| {
            next.storage_scope()
                .and_then(|scope| {
                    world
                        .get_resource::<WidgetryFileDialogStorage>()
                        .and_then(|store| store.snapshot(scope))
                })
                .is_some_and(|snapshot| snapshot.pinned.contains(path) != *add)
        });
        if !next.act(action)? && !store_pin_changes {
            return Ok(false);
        }
        if reopen
            && let Some(snapshot) = next
                .storage_scope()
                .and_then(|scope| {
                    world
                        .get_resource::<WidgetryFileDialogStorage>()
                        .and_then(|store| store.snapshot(scope))
                })
                .cloned()
        {
            next.requested_path = None;
            next.initialize_storage(snapshot);
        }
        commit(world, root, next, false, pin)
    }

    pub fn queue(commands: &mut Commands, root: Entity, action: WidgetryFileDialogAction) {
        commands.queue(move |world: &mut World| Self::apply(world, root, action).map(|_| ()));
    }

    pub fn deliver(
        world: &mut World,
        root: Entity,
        reply: crate::WidgetryFileDialogReply,
    ) -> Result<bool, BevyError> {
        if world.get::<Self>(root).is_none() {
            return Ok(false);
        }
        let mut next = world
            .get::<WidgetryFileDialogState>(root)
            .ok_or_else(|| contract_error("FileDialog state missing"))?
            .clone();
        let visited = matches!(reply, crate::WidgetryFileDialogReply::Started { .. });
        if !next.receive(reply)? {
            return Ok(false);
        }
        commit(world, root, next, visited, None)
    }
}

fn commit(
    world: &mut World,
    root: Entity,
    mut next: WidgetryFileDialogState,
    visited: bool,
    pin: Option<(std::path::PathBuf, bool)>,
) -> Result<bool, BevyError> {
    let previous = world
        .get::<WidgetryFileDialogState>(root)
        .ok_or_else(|| contract_error("FileDialog state missing"))?;
    let result = if previous.result().is_none() {
        next.result().cloned()
    } else {
        None
    };
    if let Some(result) = &result {
        next.preferences.last_picked_dir = match result {
            WidgetryFileDialogResult::File(path) | WidgetryFileDialogResult::SavePath(path) => {
                path.parent().map(ToOwned::to_owned)
            }
            WidgetryFileDialogResult::Directory(path) => Some(path.clone()),
            WidgetryFileDialogResult::Files(_) | WidgetryFileDialogResult::Directories(_) => {
                next.current_path.clone()
            }
            WidgetryFileDialogResult::Cancelled => next.preferences.last_picked_dir.clone(),
        };
    }
    let merge = if let Some(scope) = next.storage_scope() {
        let storage = world
            .get_resource::<WidgetryFileDialogStorage>()
            .ok_or_else(|| {
                contract_error("FileDialogHeadlessPlugin required for scoped storage")
            })?;
        match storage.prepare_merge(
            scope,
            previous.preferences(),
            next.preferences(),
            visited,
            result
                .as_ref()
                .is_some_and(|result| *result != WidgetryFileDialogResult::Cancelled),
            pin.as_ref(),
        ) {
            Ok(record) => {
                let scope = scope.to_owned();
                next.preferences.pinned = record.snapshot.pinned.clone();
                Some((scope, record))
            }
            Err(error) => {
                next.storage_state =
                    crate::WidgetryFileDialogStorageState::Failed(error.to_string());
                None
            }
        }
    } else {
        None
    };
    let session = next.token().session;
    world.entity_mut(root).insert(next);
    if let Some((scope, record)) = merge {
        world
            .resource_mut::<WidgetryFileDialogStorage>()
            .scopes
            .insert(scope, record);
    }
    world.trigger(WidgetryFileDialogChangeEvent {
        entity: root,
        session,
    });
    if let Some(result) = result {
        world.trigger(WidgetryFileDialogResultEvent {
            entity: root,
            session,
            result,
        });
    }
    Ok(true)
}
