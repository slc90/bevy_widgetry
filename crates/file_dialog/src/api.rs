use crate::{
    FileDialogApiFault, FileDialogError, FileDialogFailure, FileDialogOperationFailure,
    WidgetryFileDialogAction, WidgetryFileDialogProps, WidgetryFileDialogResult,
    WidgetryFileDialogSessionId, WidgetryFileDialogState, WidgetryFileDialogStorage,
};
use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

#[derive(SceneComponent, FromTemplate, Default)]
#[scene(WidgetryFileDialogProps)]
#[require(crate::colors::ColorState)]
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
        crate::style::install(app);
        crate::window::install(app);
        widgetry_info!("WidgetryFileDialogPlugin 注册完成");
    }
}

impl Plugin for WidgetryFileDialogHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_job::JobManagerPlugin>() {
            app.add_plugins(bevy_widgetry_job::JobManagerPlugin);
        }
        app.init_resource::<WidgetryFileDialogStorage>();
        app.init_resource::<crate::WidgetryFileDialogBackend>()
            .init_resource::<crate::WidgetryFileDialogRuntimeOptions>()
            .init_resource::<crate::WidgetryFileDialogRuntimeStatus>()
            .init_resource::<crate::runtime::Runtime>()
            .init_resource::<crate::runtime::FlushRequest>()
            .add_systems(Update, crate::runtime::update);
        widgetry_info!("WidgetryFileDialogHeadlessPlugin 注册完成");
    }
}

impl WidgetryFileDialog {
    fn scene(props: WidgetryFileDialogProps) -> impl Scene {
        let window = props.window.clone();
        let colors = props.colors.clone();
        (
            bsn! {
                template(move |_| colors.clone().initial())
                template(move |context| {
                    if let Some(state) = context.entity.get::<WidgetryFileDialogState>() { return Ok(state.clone()); }
                    let mut state = WidgetryFileDialogState::new(props.clone())
                        .map_err(|failure| failure.with_context(bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.scene").field("root", context.entity.id())).into_bevy_error())?;
                    if let Some(snapshot) = state.storage_scope().and_then(|scope|
                        context.entity.world().get_resource::<WidgetryFileDialogStorage>()
                            .and_then(|storage| storage.snapshot(scope))).cloned() {
                        state.initialize_storage(snapshot);
                    }
                    context.entity.world_scope(|world| crate::runtime::wake(world)).map_err(FileDialogFailure::into_bevy_error)?;
                    Ok(state)
                })
                WidgetryFileDialog
            },
            crate::window::scene(window),
        )
    }

    pub fn apply(
        world: &mut World,
        root: Entity,
        action: WidgetryFileDialogAction,
    ) -> Result<bool, FileDialogOperationFailure> {
        if world.get::<Self>(root).is_none() {
            return Err(api_failure(
                world,
                Some(root),
                Some(action),
                FileDialogApiFault::InvalidRoot,
            )
            .into());
        }
        if action == WidgetryFileDialogAction::Reopen && crate::window::is_independent(world, root)
        {
            return Err(api_failure(
                world,
                Some(root),
                Some(action),
                FileDialogApiFault::IndependentReopen,
            )
            .into());
        }
        let mut next = world
            .get::<WidgetryFileDialogState>(root)
            .ok_or_else(|| {
                api_failure(
                    world,
                    Some(root),
                    Some(action.clone()),
                    FileDialogApiFault::MissingState,
                )
            })?
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
        let frame = bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.apply")
            .field("root", root)
            .field("action", format_args!("{action:?}"));
        if !next
            .act(action)
            .map_err(|failure| failure.with_context(frame))?
            && !store_pin_changes
        {
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
        let changed = commit(world, root, next, false, pin)?;
        if changed {
            crate::runtime::wake(world)?;
        }
        Ok(changed)
    }

    pub fn queue(commands: &mut Commands, root: Entity, action: WidgetryFileDialogAction) {
        commands.queue(move |world: &mut World| {
            Self::apply(world, root, action)
                .map(|_| ())
                .map_err(FileDialogOperationFailure::into_bevy_error)
        });
    }

    pub fn flush_preferences(world: &mut World) -> Result<(), FileDialogOperationFailure> {
        if !world.contains_resource::<crate::WidgetryFileDialogPersistence>() {
            return Err(
                api_failure(world, None, None, FileDialogApiFault::MissingPersistence).into(),
            );
        }
        // Reply commit may run observer commands while Runtime is outside World processing replies.
        // Keep flush admission separately in World until the I/O service actually accepts the request.
        if !world.contains_resource::<crate::runtime::FlushRequest>() {
            return Err(api_failure(world, None, None, FileDialogApiFault::MissingRuntime).into());
        }
        world.resource_mut::<crate::runtime::FlushRequest>().0 = true;
        crate::runtime::wake(world).map_err(Into::into)
    }

    pub fn deliver(
        world: &mut World,
        root: Entity,
        reply: crate::WidgetryFileDialogReply,
    ) -> Result<bool, FileDialogOperationFailure> {
        if world.get::<Self>(root).is_none() {
            return Ok(false);
        }
        let mut next = world
            .get::<WidgetryFileDialogState>(root)
            .ok_or_else(|| api_failure(world, Some(root), None, FileDialogApiFault::MissingState))?
            .clone();
        let visited = matches!(reply, crate::WidgetryFileDialogReply::Started { .. });
        if !next.receive(reply).map_err(|failure| {
            failure.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("file_dialog.deliver")
                    .field("root", root),
            )
        })? {
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
) -> Result<bool, FileDialogOperationFailure> {
    let previous = world
        .get::<WidgetryFileDialogState>(root)
        .ok_or_else(|| api_failure(world, Some(root), None, FileDialogApiFault::MissingState))?;
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
    let mut merge_failure = None;
    let merge = if let Some(scope) = next.storage_scope() {
        let storage = world
            .get_resource::<WidgetryFileDialogStorage>()
            .ok_or_else(|| {
                api_failure(world, Some(root), None, FileDialogApiFault::MissingStorage)
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
            Ok(mut record) => {
                if !world.contains_resource::<crate::WidgetryFileDialogPersistence>() {
                    record.pending_load = false;
                    record.mutations = crate::storage::StorageMutations::default();
                }
                let scope = scope.to_owned();
                next.preferences.pinned = record.snapshot.pinned.clone();
                Some((scope, record))
            }
            Err(error) => {
                next.storage_state =
                    crate::WidgetryFileDialogStorageState::Failed(error.to_string());
                merge_failure = Some(error);
                None
            }
        }
    } else {
        None
    };
    let invalidated = previous.token().session != next.token().session
        || previous.token().generation != next.token().generation
        || next.session_state() != crate::WidgetryFileDialogSessionState::Open;
    if invalidated
        && let Some(mut runtime) = world.get_resource_mut::<crate::runtime::Runtime>()
        && let Err(error) = runtime.cancel(root)
    {
        let mut failure = bevy_widgetry_job::JobFailure::from(error);
        if let Some(merge) = merge_failure.take() {
            failure = failure.combine(merge.into());
        }
        return Err(FileDialogOperationFailure::Job(failure));
    }
    let session = next.token().session;
    world.entity_mut(root).insert(next);
    if let Some((scope, record)) = merge {
        world
            .resource_mut::<WidgetryFileDialogStorage>()
            .scopes
            .insert(scope, record);
    }
    if let Some(failure) = merge_failure {
        failure.record_handled(
            "终止本次偏好合并并保留原存储",
            "已提交 FileDialog 存储失败状态，其他会话状态照常提交",
        );
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

fn api_failure(
    world: &World,
    root: Option<Entity>,
    action: Option<WidgetryFileDialogAction>,
    fault: FileDialogApiFault,
) -> FileDialogFailure {
    let token = root
        .and_then(|root| world.get::<WidgetryFileDialogState>(root))
        .map(WidgetryFileDialogState::token);
    crate::error::failure(
        FileDialogError::Api {
            root,
            token,
            action: action.map(Box::new),
            fault,
        },
        "file_dialog.api",
    )
}
