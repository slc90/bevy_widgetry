#[allow(unsafe_code)]
mod native;

use crate::{
    WidgetryFileDialog, WidgetryFileDialogEntry, WidgetryFileDialogEntryKind,
    WidgetryFileDialogRuntimeOptions, WidgetryFileDialogSessionId, WidgetryFileDialogSessionState,
    WidgetryFileDialogState,
};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_core::{icon::WidgetryIcon, scene::ErrorFrame};
use bevy_widgetry_job::{
    JobFailure, JobId, JobManager, JobOptions, JobOutcome, JobRead, JobReceiver, SendStatus,
    SubmitFailure,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const SIZE: u32 = 32;
const CAPACITY: usize = 512;
const BATCH: usize = 16;
const RESULTS_PER_UPDATE: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum IconKey {
    Folder,
    FileType(OsString),
    Path(PathBuf),
}

impl IconKey {
    fn for_path(path: &Path, kind: WidgetryFileDialogEntryKind) -> Option<Self> {
        if kind.is_directory() {
            return Some(Self::Folder);
        }
        if !kind.is_file() {
            return None;
        }
        let extension = path.extension().unwrap_or_default().to_ascii_lowercase();
        if kind == WidgetryFileDialogEntryKind::SymlinkFile
            || ["exe", "ico", "lnk", "url", "cpl", "scr"]
                .iter()
                .any(|suffix| extension == *suffix)
        {
            Some(Self::Path(path.to_owned()))
        } else {
            Some(Self::FileType(extension))
        }
    }
}

#[derive(Clone)]
struct Pixels(Vec<u8>);

#[derive(Debug)]
struct IconError {
    key: Option<IconKey>,
    operation: &'static str,
    source: std::io::Error,
}

impl std::fmt::Display for IconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FileDialog 原生图标失败：{} {:?}",
            self.operation, self.key
        )
    }
}

impl std::error::Error for IconError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

fn failure(key: Option<IconKey>, operation: &'static str, source: std::io::Error) -> JobFailure {
    bevy_widgetry_diagnostics::WidgetryFailure::new(
        IconError {
            key,
            operation,
            source,
        },
        ErrorFrame::new(operation),
    )
    .into()
}

// Rendering against two backgrounds recovers alpha for both legacy mask icons and alpha icons.
// GDI's unused alpha bytes are not reliable for legacy icons; Bevy needs straight RGBA.
fn straight_rgba(black: &[u8], white: &[u8]) -> Vec<u8> {
    black
        .as_chunks::<4>()
        .0
        .iter()
        .zip(white.as_chunks::<4>().0.iter())
        .flat_map(|(black, white)| {
            let transparency = (0..3)
                .map(|i| white[i].saturating_sub(black[i]))
                .max()
                .unwrap_or(255);
            let alpha = 255 - transparency;
            let channel = |index| {
                if alpha == 0 {
                    0
                } else {
                    ((u32::from(black[index]) * 255 + u32::from(alpha) / 2) / u32::from(alpha))
                        .min(255) as u8
                }
            };
            [channel(2), channel(1), channel(0), alpha]
        })
        .collect()
}

enum Cached {
    Loading,
    Ready(Handle<Image>),
    Unavailable,
}

#[derive(Default)]
struct Cache {
    entries: HashMap<IconKey, Cached>,
    order: VecDeque<IconKey>,
}

impl Cache {
    fn reserve(&mut self, visible: &[IconKey]) -> Vec<IconKey> {
        let protected = visible.iter().collect::<HashSet<_>>();
        let mut requests = Vec::new();
        for key in visible {
            if self.entries.contains_key(key) {
                continue;
            }
            if self.entries.len() == CAPACITY {
                let candidate = self.order.iter().position(|old| {
                    !protected.contains(old)
                        && !matches!(self.entries.get(old), Some(Cached::Loading))
                });
                let Some(index) = candidate else {
                    break;
                };
                if let Some(old) = self.order.remove(index) {
                    self.entries.remove(&old);
                }
            }
            self.entries.insert(key.clone(), Cached::Loading);
            self.order.push_back(key.clone());
            requests.push(key.clone());
            if requests.len() == BATCH {
                break;
            }
        }
        requests
    }

    fn image(&self, key: &IconKey) -> Option<Handle<Image>> {
        match self.entries.get(key) {
            Some(Cached::Ready(image)) => Some(image.clone()),
            _ => None,
        }
    }
}

struct Reply {
    values: Vec<(IconKey, Option<Pixels>)>,
    failure: Option<JobFailure>,
}

struct Worker {
    manager: JobManager,
    id: Option<JobId>,
    receiver: JobReceiver<Reply>,
    sender: crossbeam_channel::Sender<Vec<IconKey>>,
    active: Vec<IconKey>,
    pending: Option<(std::sync::Arc<Reply>, usize)>,
    failure_handled: bool,
}

impl Worker {
    fn busy(&self) -> bool {
        !self.active.is_empty() || self.pending.is_some()
    }

    fn request(
        &mut self,
        keys: Vec<IconKey>,
    ) -> std::result::Result<(), crossbeam_channel::TrySendError<Vec<IconKey>>> {
        if keys.is_empty() {
            return Ok(());
        }
        self.sender.try_send(keys.clone())?;
        self.active = keys;
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            crate::runtime::abandon_job(self.manager.clone(), id);
        }
    }
}

fn spawn_worker(
    manager: &JobManager,
    mut extract: impl FnMut(&IconKey) -> std::io::Result<Option<Pixels>> + Send + 'static,
) -> std::result::Result<Worker, SubmitFailure> {
    let (sender, requests) = crossbeam_channel::bounded::<Vec<IconKey>>(1);
    let (id, receiver) = manager.spawn_thread(
        JobOptions {
            label: Some("FileDialog Shell icons".into()),
            buffer_capacity: Some(1),
        },
        move |context| {
            bevy::tasks::block_on(async move {
                if crate::retention::cancel_if_requested(&context) {
                    return Ok(());
                }
                let _apartment = native::Apartment::new()
                    .map_err(|error| failure(None, "file_dialog.icons.com_initialize", error))?;
                loop {
                    if crate::retention::cancel_if_requested(&context) {
                        return Ok(());
                    }
                    let keys = match requests.recv_timeout(std::time::Duration::from_millis(10)) {
                        Ok(keys) => keys,
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
                        Err(error) => {
                            if crate::retention::cancel_if_requested(&context) {
                                return Ok(());
                            }
                            return Err(failure(
                                None,
                                "file_dialog.icons.request_receive",
                                std::io::Error::other(error),
                            ));
                        }
                    };
                    let mut failures: Option<JobFailure> = None;
                    let mut values = Vec::with_capacity(keys.len());
                    for key in keys {
                        if crate::retention::cancel_if_requested(&context) {
                            return failures.map_or(Ok(()), Err);
                        }
                        let pixels = match extract(&key) {
                            Ok(pixels) => pixels,
                            Err(error) => {
                                let error =
                                    failure(Some(key.clone()), "file_dialog.icons.extract", error);
                                failures = Some(match failures {
                                    Some(previous) => previous.combine(error),
                                    None => error,
                                });
                                None
                            }
                        };
                        values.push((key, pixels));
                    }
                    match context
                        .send(Reply {
                            values,
                            failure: failures.clone(),
                        })
                        .await
                    {
                        Ok(SendStatus::Sent) => {}
                        Ok(SendStatus::Canceled) => return failures.map_or(Ok(()), Err),
                        Err(error) => {
                            let error: JobFailure = error.into();
                            return Err(match failures {
                                Some(previous) => previous.combine(error),
                                None => error,
                            });
                        }
                    }
                    if let Some(error) = failures {
                        return Err(error);
                    }
                }
            })
        },
    )?;
    Ok(Worker {
        manager: manager.clone(),
        id: Some(id),
        receiver,
        sender,
        active: Vec::new(),
        pending: None,
        failure_handled: false,
    })
}

#[derive(Component)]
struct IconState {
    identity: (WidgetryFileDialogSessionId, u64),
    cache: Cache,
    worker: Option<Worker>,
    stopped: bool,
    broken: bool,
}

impl IconState {
    fn new(identity: (WidgetryFileDialogSessionId, u64)) -> Self {
        Self {
            identity,
            cache: Cache::default(),
            worker: None,
            stopped: false,
            broken: false,
        }
    }

    fn poll(&mut self, world: &mut World, budget: &mut usize) -> Result {
        let Some(worker) = &mut self.worker else {
            return Ok(());
        };
        while *budget > 0 {
            if let Some((reply, index)) = &mut worker.pending {
                let (key, pixels) = reply.values.get(*index).ok_or_else(|| {
                    crate::error::internal_error(crate::FileDialogInternalFault::Missing {
                        part: "icon result index",
                    })
                })?;
                let image = if let Some(Pixels(bytes)) = pixels {
                    if bytes.len() != (SIZE * SIZE * 4) as usize {
                        return Err(failure(
                            Some(key.clone()),
                            "file_dialog.icons.pixels",
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                "invalid RGBA icon buffer",
                            ),
                        )
                        .into_bevy_error());
                    }
                    let mut images =
                        world.get_resource_mut::<Assets<Image>>().ok_or_else(|| {
                            crate::error::internal_error(crate::FileDialogInternalFault::Missing {
                                part: "icon image assets",
                            })
                        })?;
                    Some(images.add(Image::new(
                        Extent3d {
                            width: SIZE,
                            height: SIZE,
                            depth_or_array_layers: 1,
                        },
                        TextureDimension::D2,
                        bytes.clone(),
                        TextureFormat::Rgba8UnormSrgb,
                        RenderAssetUsages::default(),
                    )))
                } else {
                    None
                };
                self.cache.entries.insert(
                    key.clone(),
                    image.map(Cached::Ready).unwrap_or(Cached::Unavailable),
                );
                *index += 1;
                *budget -= 1;
                if *index == reply.values.len() {
                    if let Some(error) = &reply.failure {
                        self.stopped = true;
                        error.record_handled(
                            "文件图标提取失败，保留占位图标",
                            "文件选择业务保持可用",
                        );
                        worker.failure_handled = true;
                    }
                    worker.pending = None;
                    worker.active.clear();
                }
                continue;
            }
            match worker.receiver.try_recv() {
                Ok(JobRead::Empty) => return Ok(()),
                Ok(JobRead::Item(reply)) => {
                    if reply.values.is_empty() {
                        worker.active.clear();
                    } else {
                        worker.pending = Some((reply, 0));
                    }
                }
                Ok(JobRead::Finished(outcome)) => {
                    for key in &worker.active {
                        self.cache.entries.insert(key.clone(), Cached::Unavailable);
                    }
                    self.stopped = true;
                    if let JobOutcome::Failed(error) = &outcome
                        && !worker.failure_handled
                    {
                        error.record_handled("图标服务失败，保留占位图标", "文件选择业务保持可用");
                    }
                    if let Some(id) = worker.id {
                        worker
                            .manager
                            .remove_if_present(id)
                            .map_err(|error| JobFailure::from(error).into_bevy_error())?;
                        worker.id = None;
                    }
                    self.worker = None;
                    return Ok(());
                }
                Ok(JobRead::Closed) => {
                    for key in &worker.active {
                        self.cache.entries.insert(key.clone(), Cached::Unavailable);
                    }
                    self.worker = None;
                    self.stopped = true;
                    return Err(crate::error::internal_error(
                        crate::FileDialogInternalFault::Missing {
                            part: "icon job terminal outcome",
                        },
                    ));
                }
                Err(error) => {
                    for key in &worker.active {
                        self.cache.entries.insert(key.clone(), Cached::Unavailable);
                    }
                    self.worker = None;
                    self.stopped = true;
                    let error: JobFailure = error.into();
                    error.record_handled("停止图标接收并保留占位图标", "文件选择业务保持可用");
                    return Ok(());
                }
            }
        }
        world.write_message(bevy::window::RequestRedraw);
        Ok(())
    }
}

#[derive(Component)]
struct Slot {
    key: Option<IconKey>,
    shown: Option<AssetId<Image>>,
}

pub(crate) fn scene(entry: &WidgetryFileDialogEntry) -> impl Scene {
    let key = IconKey::for_path(entry.path(), entry.kind());
    let fallback = if entry.kind().is_directory() {
        BuiltinIcon::FileDialogFolder
    } else {
        BuiltinIcon::FileDialogFile
    };
    bsn! {
        Name("FileDialogEntryIcon") template(move |_| Ok(Slot {key:key.clone(),shown:None}))
        Node {width:px(16),height:px(16),flex_shrink:0.0} template(|_| Ok(Pickable::IGNORE))
        Children [
            @WidgetryIcon {@path:{fallback.path()},@max_size:{Some(UVec2::splat(16))}} Node {width:px(16),height:px(16)} template(|_| Ok(Pickable::IGNORE))--
            Name("FileDialogNativeIcon") ImageNode::default() Node {width:px(16),height:px(16),display:Display::None} template(|_| Ok(Pickable::IGNORE))
        ]
    }
}

fn project(world: &mut World, entity: Entity, image: Option<Handle<Image>>) -> Result {
    let shown = image.as_ref().map(Handle::id);
    if world
        .get::<Slot>(entity)
        .is_some_and(|slot| slot.shown == shown)
    {
        return Ok(());
    }
    let children = world
        .get::<Children>(entity)
        .filter(|children| children.len() == 2)
        .ok_or_else(|| {
            crate::error::internal_error(crate::FileDialogInternalFault::Missing {
                part: "icon slot children",
            })
        })?;
    let (fallback, native) = (children[0], children[1]);
    let mut node = world.get_mut::<Node>(fallback).ok_or_else(|| {
        crate::error::internal_error(crate::FileDialogInternalFault::Missing {
            part: "icon fallback node",
        })
    })?;
    node.display = if image.is_some() {
        Display::None
    } else {
        Display::Flex
    };
    let mut node = world.get_mut::<Node>(native).ok_or_else(|| {
        crate::error::internal_error(crate::FileDialogInternalFault::Missing {
            part: "native icon node",
        })
    })?;
    node.display = if image.is_some() {
        Display::Flex
    } else {
        Display::None
    };
    world
        .get_mut::<ImageNode>(native)
        .ok_or_else(|| {
            crate::error::internal_error(crate::FileDialogInternalFault::Missing {
                part: "native icon image",
            })
        })?
        .image = image.unwrap_or_default();
    if let Some(mut slot) = world.get_mut::<Slot>(entity) {
        slot.shown = shown;
    }
    world.write_message(bevy::window::RequestRedraw);
    Ok(())
}

fn update_root(world: &mut World, root: Entity, budget: &mut usize) -> Result {
    let state = world.get::<WidgetryFileDialogState>(root).ok_or_else(|| {
        crate::error::internal_error(crate::FileDialogInternalFault::Missing {
            part: "icon session state",
        })
    })?;
    if state.session_state() != WidgetryFileDialogSessionState::Open
        || !world
            .resource::<WidgetryFileDialogRuntimeOptions>()
            .automatic
    {
        world.entity_mut(root).remove::<IconState>();
        return Ok(());
    }
    let identity = (state.token().session, state.token().generation);
    let mut state = world
        .entity_mut(root)
        .take::<IconState>()
        .filter(|cache| cache.identity == identity)
        .unwrap_or_else(|| IconState::new(identity));
    if state.broken {
        world.entity_mut(root).insert(state);
        return Ok(());
    }
    let slots = world
        .query::<(Entity, &Slot, &ChildOf)>()
        .iter(world)
        .filter(|(_, _, parent)| {
            world
                .get::<crate::view::EntryRow>(parent.parent())
                .is_some_and(|row| row.root == root)
        })
        .map(|(entity, slot, _)| (entity, slot.key.clone()))
        .collect::<Vec<_>>();
    let visible = slots
        .iter()
        .filter_map(|(_, key)| key.clone())
        .collect::<Vec<_>>();
    let result = (|| {
        state.poll(world, budget)?;
        if !state.stopped && state.worker.as_ref().is_none_or(|worker| !worker.busy()) {
            let keys = state.cache.reserve(&visible);
            if !keys.is_empty() {
                if state.worker.is_none() {
                    match spawn_worker(world.resource::<JobManager>(), native::extract) {
                        Ok(worker) => state.worker = Some(worker),
                        Err(error) => {
                            for key in &keys {
                                state.cache.entries.insert(key.clone(), Cached::Unavailable);
                            }
                            state.stopped = true;
                            error
                                .with_context(
                                    ErrorFrame::new("file_dialog.icons.submit").field("root", root),
                                )
                                .record_handled(
                                    "图标任务提交失败，保留占位图标",
                                    "文件选择业务保持可用",
                                );
                        }
                    }
                }
                if let Some(worker) = &mut state.worker
                    && let Err(error) = worker.request(keys.clone())
                {
                    for key in keys {
                        state.cache.entries.insert(key, Cached::Unavailable);
                    }
                    state.stopped = true;
                    let error = failure(
                        None,
                        "file_dialog.icons.request_send",
                        std::io::Error::other(error),
                    );
                    error.record_handled(
                        "图标请求无法递送，保留占位图标",
                        "等待受管服务退出，文件选择业务保持可用",
                    );
                }
            }
        }
        for (entity, key) in slots {
            project(
                world,
                entity,
                key.as_ref().and_then(|key| state.cache.image(key)),
            )?;
        }
        Ok(())
    })();
    if result.is_err() {
        state.broken = true;
        state.worker = None;
    }
    world.entity_mut(root).insert(state);
    result
}

pub(crate) fn update(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryFileDialog>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut budget = RESULTS_PER_UPDATE;
    for root in roots {
        if let Err(error) = update_root(world, root, &mut budget) {
            let error = crate::error::boundary_context(
                error,
                crate::error::owner_frame(world, root, root, "file_dialog.icons.update"),
            );
            world
                .commands()
                .queue(move |_: &mut World| -> Result { Err(error) });
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::*;
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    fn spawn_batch(
        manager: &JobManager,
        keys: Vec<IconKey>,
        extract: impl FnMut(&IconKey) -> std::io::Result<Option<Pixels>> + Send + 'static,
    ) -> std::result::Result<Worker, SubmitFailure> {
        let mut worker = spawn_worker(manager, extract)?;
        worker.request(keys).unwrap();
        Ok(worker)
    }

    fn identity() -> (WidgetryFileDialogSessionId, u64) {
        let state = WidgetryFileDialogState::new(Default::default()).unwrap();
        (state.token().session, state.token().generation)
    }

    fn world() -> World {
        bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
        let mut world = World::new();
        world.init_resource::<Assets<Image>>();
        world.init_resource::<Messages<bevy::window::RequestRedraw>>();
        world
    }

    fn finish(state: &mut IconState, world: &mut World) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while state.worker.as_ref().is_some_and(Worker::busy) {
            let mut budget = RESULTS_PER_UPDATE;
            state.poll(world, &mut budget).unwrap();
            assert!(Instant::now() < deadline, "icon batch did not finish");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    fn pixels(value: u8) -> Pixels {
        Pixels(vec![value; (SIZE * SIZE * 4) as usize])
    }

    fn deliver(app: &mut App, root: Entity, snapshot: Arc<WidgetryFileDialogSnapshot>) {
        let selection = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .projection_selection_job(snapshot.clone())
            .unwrap()
            .prepare()
            .unwrap();
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Snapshot {
                snapshot,
                state: WidgetryFileDialogDirectoryState::Ready,
                selection,
            },
        )
        .unwrap();
    }

    fn fixture(names: &[&str]) -> (App, Entity, Arc<WidgetryFileDialogSnapshot>) {
        let mut app = bevy_widgetry_test_utils::text_input_app();
        bevy_widgetry_test_utils::add_ui_plugins(&mut app);
        bevy_widgetry_test_utils::spawn_ui_camera(&mut app, UVec2::new(1000, 700), 1.0);
        app.insert_resource(WidgetryFileDialogRuntimeOptions {
            automatic: false,
            ..default()
        });
        app.add_plugins(WidgetryFileDialogPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! {@WidgetryFileDialog Node {width:px(900),height:px(650)}})
            .unwrap()
            .id();
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let token = state.token();
        let path = PathBuf::from("C:/fixture");
        let data = names
            .iter()
            .map(|name| WidgetryFileDialogEntryData {
                path: path.join(name),
                name: (*name).into(),
                kind: WidgetryFileDialogEntryKind::File,
                size: None,
                modified: None,
                hidden: Some(false),
                system: Some(false),
            })
            .collect();
        let snapshot = Arc::new(
            WidgetryFileDialogSnapshot::prepare(token, path.clone(), data, &state.query(), None)
                .unwrap(),
        );
        WidgetryFileDialog::deliver(
            app.world_mut(),
            root,
            WidgetryFileDialogReply::Started { token, path },
        )
        .unwrap();
        deliver(&mut app, root, snapshot.clone());
        for _ in 0..4 {
            app.update();
        }
        (app, root, snapshot)
    }

    #[test]
    fn late_result_cannot_overwrite_a_recycled_row_and_rebinding_starts_with_a_placeholder() {
        let (mut app, root, snapshot) = fixture(&["first.exe", "second.exe"]);
        let first = IconKey::Path("C:/fixture/first.exe".into());
        let (slot, row) = app
            .world_mut()
            .query::<(Entity, &Slot, &ChildOf)>()
            .iter(app.world())
            .find(|(_, slot, _)| slot.key == Some(first.clone()))
            .map(|(entity, _, parent)| (entity, parent.parent()))
            .unwrap();
        let old_image = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::new(
                Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels(20).0,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ));
        project(app.world_mut(), slot, Some(old_image)).unwrap();
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        let mut state = IconState::new((token.session, token.generation));
        let keys = state.cache.reserve(std::slice::from_ref(&first));
        let (release, gate) = mpsc::channel();
        let (entered, ready) = mpsc::channel();
        state.worker = Some(
            spawn_batch(app.world().resource::<JobManager>(), keys, move |_| {
                entered.send(()).unwrap();
                gate.recv().unwrap();
                Ok(Some(pixels(77)))
            })
            .unwrap(),
        );
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Search("second".into()),
        )
        .unwrap();
        let current = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        let projected = Arc::new(
            snapshot
                .reproject(current.token(), &current.query())
                .unwrap(),
        );
        deliver(&mut app, root, projected);
        app.update();
        assert!(!app.world().entities().contains(slot));
        let current_slot = app
            .world_mut()
            .query::<(Entity, &Slot, &ChildOf)>()
            .iter(app.world())
            .find(|(_, _, parent)| parent.parent() == row)
            .map(|(entity, _, _)| entity)
            .unwrap();
        let native = app.world().get::<Children>(current_slot).unwrap()[1];
        assert_eq!(
            app.world().get::<Node>(native).unwrap().display,
            Display::None
        );
        release.send(()).unwrap();
        finish(&mut state, app.world_mut());
        assert!(state.cache.image(&first).is_some());
        // Drive the same production root projection with the late first-file cache and the new binding.
        state.stopped = true;
        app.world_mut().entity_mut(root).insert(state);
        app.world_mut()
            .resource_mut::<WidgetryFileDialogRuntimeOptions>()
            .automatic = true;
        let mut budget = RESULTS_PER_UPDATE;
        update_root(app.world_mut(), root, &mut budget).unwrap();
        assert_eq!(
            app.world().get::<Node>(native).unwrap().display,
            Display::None
        );
        assert!(
            app.world()
                .get::<Slot>(current_slot)
                .unwrap()
                .shown
                .is_none()
        );
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .search(),
            "second"
        );
    }

    #[test]
    fn query_changes_reuse_cache_but_navigation_and_reopen_drop_old_icon_generations() {
        let (mut app, root, _) = fixture(&["first.txt"]);
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        let mut icons = IconState::new((token.session, token.generation));
        icons
            .cache
            .entries
            .insert(IconKey::FileType("txt".into()), Cached::Unavailable);
        icons.stopped = true;
        app.world_mut().entity_mut(root).insert(icons);
        app.world_mut()
            .resource_mut::<WidgetryFileDialogRuntimeOptions>()
            .automatic = true;
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Search("first".into()),
        )
        .unwrap();
        let mut budget = RESULTS_PER_UPDATE;
        update_root(app.world_mut(), root, &mut budget).unwrap();
        assert!(
            app.world()
                .get::<IconState>(root)
                .unwrap()
                .cache
                .entries
                .contains_key(&IconKey::FileType("txt".into()))
        );
        WidgetryFileDialog::apply(
            app.world_mut(),
            root,
            WidgetryFileDialogAction::Navigate(WidgetryFileDialogNavigation::Path(
                "C:/other".into(),
            )),
        )
        .unwrap();
        update_root(app.world_mut(), root, &mut 0).unwrap();
        assert_ne!(
            app.world().get::<IconState>(root).unwrap().identity,
            (token.session, token.generation)
        );
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Cancel).unwrap();
        update_root(app.world_mut(), root, &mut 0).unwrap();
        assert!(app.world().get::<IconState>(root).is_none());
        WidgetryFileDialog::apply(app.world_mut(), root, WidgetryFileDialogAction::Reopen).unwrap();
        update_root(app.world_mut(), root, &mut 0).unwrap();
        assert_ne!(
            app.world().get::<IconState>(root).unwrap().identity.0,
            token.session
        );
    }

    #[test]
    fn manager_shutdown_degrades_icons_once_without_changing_dialog_authority() {
        let (mut app, root, _) = fixture(&["first.txt"]);
        let before = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        app.world()
            .resource::<JobManager>()
            .begin_shutdown(Duration::from_secs(1))
            .unwrap();
        app.world_mut()
            .resource_mut::<WidgetryFileDialogRuntimeOptions>()
            .automatic = true;
        for _ in 0..3 {
            let mut budget = RESULTS_PER_UPDATE;
            update_root(app.world_mut(), root, &mut budget).unwrap();
        }
        assert!(app.world().get::<IconState>(root).unwrap().stopped);
        let state = app.world().get::<WidgetryFileDialogState>(root).unwrap();
        assert_eq!(state.token(), before);
        assert!(state.result().is_none());
        assert!(state.error().is_none());
    }

    #[test]
    fn broken_presentation_reports_once_at_the_command_boundary_without_restarting_the_service() {
        let (mut app, root, _) = fixture(&["first.txt"]);
        app.set_error_handler(bevy_widgetry_test_utils::ErrorCapture::handler());
        let token = app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .unwrap()
            .token();
        let mut state = IconState::new((token.session, token.generation));
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        state
            .cache
            .entries
            .insert(IconKey::FileType("txt".into()), Cached::Ready(handle));
        state.stopped = true;
        app.world_mut().entity_mut(root).insert(state);
        app.world_mut()
            .resource_mut::<WidgetryFileDialogRuntimeOptions>()
            .automatic = true;
        let native = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find(|(_, name)| name.as_str() == "FileDialogNativeIcon")
            .unwrap()
            .0;
        app.world_mut().entity_mut(native).remove::<ImageNode>();
        let errors = bevy_widgetry_test_utils::ErrorCapture::default();
        errors.run(|| {
            update(app.world_mut());
            app.world_mut().flush();
            update(app.world_mut());
            app.world_mut().flush();
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        assert!(records[0].managed().is_some());
        assert!(app.world().get::<IconState>(root).unwrap().broken);
        assert!(app.world().get::<IconState>(root).unwrap().worker.is_none());
        assert_eq!(
            app.world()
                .get::<WidgetryFileDialogState>(root)
                .unwrap()
                .token(),
            token
        );
    }

    #[test]
    fn cache_deduplicates_loading_and_failed_types_and_bounds_images_without_evicting_visible_keys()
    {
        let mut cache = Cache::default();
        let common = IconKey::FileType("txt".into());
        assert_eq!(
            cache.reserve(&vec![common.clone(); 256]),
            vec![common.clone()]
        );
        assert!(cache.reserve(std::slice::from_ref(&common)).is_empty());
        cache.entries.insert(common.clone(), Cached::Unavailable);
        assert!(cache.reserve(std::slice::from_ref(&common)).is_empty());
        for index in 0..CAPACITY + 32 {
            let key = IconKey::Path(PathBuf::from(format!("C:/app-{index}.exe")));
            let requests = cache.reserve(&[common.clone(), key]);
            for key in requests {
                cache.entries.insert(key, Cached::Unavailable);
            }
            assert!(cache.entries.len() <= CAPACITY);
            assert!(cache.entries.contains_key(&common));
        }
        assert_eq!(cache.order.len(), cache.entries.len());
        assert!(
            !cache
                .entries
                .contains_key(&IconKey::Path("C:/app-0.exe".into()))
        );
    }

    #[test]
    fn alpha_reconstruction_handles_transparency_legacy_masks_and_premultiplied_color() {
        let black = [0, 0, 0, 0, 10, 20, 30, 0, 25, 50, 100, 128];
        let white = [255, 255, 255, 0, 10, 20, 30, 0, 152, 177, 227, 128];
        assert_eq!(
            straight_rgba(&black, &white),
            vec![0, 0, 0, 0, 30, 20, 10, 255, 199, 100, 50, 128]
        );
    }

    #[test]
    fn batches_reuse_one_managed_thread_and_publish_with_a_per_update_budget() {
        let mut world = world();
        let manager = JobManager::default();
        let mut state = IconState::new(identity());
        let keys = (0..BATCH)
            .map(|index| IconKey::FileType(format!("type{index}").into()))
            .collect::<Vec<_>>();
        let requested = state.cache.reserve(&keys);
        let threads = Arc::new(Mutex::new(Vec::new()));
        let observed = threads.clone();
        state.worker = Some(
            spawn_batch(&manager, requested, move |_| {
                observed.lock().unwrap().push(std::thread::current().id());
                Ok(Some(pixels(127)))
            })
            .unwrap(),
        );
        let id = state.worker.as_ref().unwrap().id.unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while threads.lock().unwrap().len() < BATCH {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(
            manager.status(id).unwrap().kind,
            bevy_widgetry_job::JobKind::Thread
        );
        state.poll(&mut world, &mut 1).unwrap();
        assert!(state.cache.image(&keys[0]).is_some());
        assert!(matches!(state.cache.entries[&keys[1]], Cached::Loading));
        finish(&mut state, &mut world);
        assert_eq!(world.resource::<Assets<Image>>().len(), BATCH);
        assert!(keys.iter().all(|key| state.cache.image(key).is_some()));
        let threads = threads.lock().unwrap();
        assert_eq!(threads.len(), BATCH);
        assert!(
            threads
                .iter()
                .all(|thread| *thread == threads[0] && *thread != std::thread::current().id())
        );
        assert_eq!(
            manager.status(id).unwrap().state,
            bevy_widgetry_job::JobState::Running
        );
        assert!(state.cache.reserve(&keys).is_empty());
        drop(threads);
        let next = IconKey::FileType("another".into());
        state.cache.reserve(std::slice::from_ref(&next));
        state
            .worker
            .as_mut()
            .unwrap()
            .request(vec![next.clone()])
            .unwrap();
        finish(&mut state, &mut world);
        assert!(state.cache.image(&next).is_some());
        assert_eq!(state.worker.as_ref().unwrap().id, Some(id));
        assert_eq!(world.resource::<Assets<Image>>().len(), BATCH + 1);
        drop(state);
        let deadline = Instant::now() + Duration::from_secs(5);
        while manager.find_status(id).is_some() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn unavailable_and_failed_extractions_keep_typed_source_and_do_not_retry_or_fail_the_session() {
        let mut world = world();
        let manager = JobManager::default();
        let mut state = IconState::new(identity());
        let keys = vec![
            IconKey::FileType("none".into()),
            IconKey::FileType("failure".into()),
        ];
        let requested = state.cache.reserve(&keys);
        state.worker = Some(
            spawn_batch(&manager, requested, |key| {
                if *key == IconKey::FileType("failure".into()) {
                    Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
                } else {
                    Ok(None)
                }
            })
            .unwrap(),
        );
        let id = state.worker.as_ref().unwrap().id.unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
        let JobOutcome::Failed(error) = manager.outcome(id).unwrap().unwrap() else {
            panic!("expected failed icon job")
        };
        let diagnostic = error.diagnostics().next().unwrap();
        let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(diagnostic);
        let mut typed = false;
        while let Some(error) = cause {
            if let Some(error) = error.downcast_ref::<IconError>() {
                assert_eq!(error.source.kind(), std::io::ErrorKind::PermissionDenied);
                assert_eq!(error.key, Some(keys[1].clone()));
                typed = true;
            }
            cause = error.source();
        }
        assert!(typed);
        finish(&mut state, &mut world);
        assert!(
            keys.iter()
                .all(|key| matches!(state.cache.entries.get(key), Some(Cached::Unavailable)))
        );
        assert!(state.cache.reserve(&keys).is_empty());
        assert_eq!(world.resource::<Assets<Image>>().len(), 0);
    }

    #[test]
    fn dropping_the_owner_cancels_and_reaps_a_blocked_batch_without_another_ui_update() {
        let mut world = world();
        let manager = JobManager::default();
        let (release, gate) = mpsc::channel();
        let (entered, ready) = mpsc::channel();
        let batch = spawn_batch(&manager, vec![IconKey::Folder], move |_| {
            entered.send(()).unwrap();
            gate.recv().unwrap();
            Ok(Some(pixels(64)))
        })
        .unwrap();
        let id = batch.id.unwrap();
        let mut state = IconState::new(identity());
        state.worker = Some(batch);
        let owner = world.spawn(state).id();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let (sentinel, check) = mpsc::channel();
        bevy::tasks::IoTaskPool::get()
            .spawn(async move {
                sentinel.send(()).unwrap();
            })
            .detach();
        check.recv_timeout(Duration::from_secs(5)).unwrap();
        world.despawn(owner);
        assert!(matches!(
            manager.status(id).unwrap().state,
            bevy_widgetry_job::JobState::Stopping
        ));
        release.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while manager.find_status(id).is_some() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn shell_extracts_current_system_types_and_folder_pixels_and_handles_unavailable_paths() {
        std::thread::spawn(|| {
            let _apartment = native::Apartment::new().unwrap();
            for key in [
                IconKey::Folder,
                IconKey::FileType("txt".into()),
                IconKey::FileType("pdf".into()),
            ] {
                for _ in 0..5 {
                    let Pixels(bytes) = native::extract(&key).unwrap().unwrap();
                    assert_eq!(bytes.len(), (SIZE * SIZE * 4) as usize);
                    assert!(bytes.as_chunks::<4>().0.iter().any(|pixel| pixel[3] > 0));
                    assert!(bytes.as_chunks::<4>().0.iter().any(|pixel| pixel[3] == 0));
                }
            }
            assert!(
                native::extract(&IconKey::Path("C:/widgetry-not-existing-4bde.ico".into()))
                    .unwrap()
                    .is_none()
            );
        })
        .join()
        .unwrap();
    }

    #[test]
    fn shared_types_and_file_specific_icons_keep_their_own_identity() {
        use std::os::windows::ffi::OsStringExt;
        let key =
            |path: &str| IconKey::for_path(Path::new(path), WidgetryFileDialogEntryKind::File);
        assert_eq!(key("C:/first.TXT"), key("D:/second.txt"));
        assert_ne!(key("C:/first.txt"), key("C:/first.pdf"));
        assert_eq!(key("C:/first.txt"), Some(IconKey::FileType("txt".into())));
        for suffix in ["exe", "ico", "lnk", "url", "cpl", "scr"] {
            assert_ne!(
                key(&format!("C:/first.{suffix}")),
                key(&format!("C:/second.{suffix}"))
            );
        }
        assert_eq!(
            IconKey::for_path(
                Path::new("C:/folder"),
                WidgetryFileDialogEntryKind::Directory
            ),
            Some(IconKey::Folder)
        );
        assert!(
            IconKey::for_path(Path::new("C:/missing"), WidgetryFileDialogEntryKind::Failed)
                .is_none()
        );
        let mut units = "C:/".encode_utf16().collect::<Vec<_>>();
        units.push(0xd800);
        units.extend(".exe".encode_utf16());
        let path = PathBuf::from(OsString::from_wide(&units));
        assert_eq!(
            IconKey::for_path(&path, WidgetryFileDialogEntryKind::File),
            Some(IconKey::Path(path))
        );
    }

    #[test]
    #[ignore = "measured dev-opt1 benchmark; run explicitly with --ignored --nocapture"]
    fn measure_native_loading_and_cached_visible_updates() {
        use bevy_widgetry_test_utils::benchmark::artifact::Artifact;
        use serde_json::json;
        let artifact = Artifact::new("file_dialog_shell_icons", "dev-opt1").unwrap();
        let extensions = [
            "txt", "pdf", "rs", "zip", "json", "md", "png", "jpg", "docx", "xlsx", "pptx", "dll",
            "mp3", "html", "bat", "xml",
        ];
        let mut reports = Vec::new();
        for count in [1000, 10000] {
            let names = (0..count)
                .map(|index| format!("file-{index:05}.{}", extensions[index % extensions.len()]))
                .collect::<Vec<_>>();
            let borrowed = names.iter().map(String::as_str).collect::<Vec<_>>();
            let mut loading = Vec::new();
            let mut cached = Vec::new();
            let mut native_wall = Vec::new();
            let mut visible = 0;
            for sample in 0..24 {
                let (mut app, root, _) = fixture(&borrowed);
                app.world_mut()
                    .resource_mut::<WidgetryFileDialogRuntimeOptions>()
                    .automatic = true;
                visible = app.world_mut().query::<&Slot>().iter(app.world()).count();
                let wall = Instant::now();
                let deadline = wall + Duration::from_secs(5);
                loop {
                    let start = Instant::now();
                    let mut budget = RESULTS_PER_UPDATE;
                    update_root(app.world_mut(), root, &mut budget).unwrap();
                    let duration = start.elapsed().as_secs_f64() * 1000.0;
                    if sample >= 4 {
                        loading.push(duration);
                    }
                    if app
                        .world()
                        .get::<IconState>(root)
                        .unwrap()
                        .worker
                        .as_ref()
                        .is_none_or(|worker| !worker.busy())
                    {
                        break;
                    }
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(1));
                }
                if sample >= 4 {
                    native_wall.push(wall.elapsed().as_secs_f64() * 1000.0);
                }
                assert!(
                    app.world()
                        .get::<IconState>(root)
                        .unwrap()
                        .cache
                        .entries
                        .values()
                        .all(|value| matches!(value, Cached::Ready(_)))
                );
                for _ in 0..64 {
                    let start = Instant::now();
                    let mut budget = RESULTS_PER_UPDATE;
                    update_root(app.world_mut(), root, &mut budget).unwrap();
                    if sample >= 4 {
                        cached.push(start.elapsed().as_secs_f64() * 1000.0);
                    }
                    assert!(
                        app.world()
                            .get::<IconState>(root)
                            .unwrap()
                            .worker
                            .as_ref()
                            .is_none_or(|worker| !worker.busy())
                    );
                }
            }
            fn stats(mut samples: Vec<f64>) -> serde_json::Value {
                let raw = samples.clone();
                samples.sort_by(f64::total_cmp);
                let n = samples.len();
                json!({"n":n,"median_ms":samples[n/2],"p95_ms":samples[(n*95).div_ceil(100)-1],"max_ms":samples[n-1],"samples_ms":raw})
            }
            reports.push(json!({"directory_entries":count,"visible_slots":visible,"loading_main_thread":stats(loading),"cached_main_thread":stats(cached),"batch_wall_including_thread_com_and_1ms_polling":stats(native_wall)}));
        }
        artifact.write_json("icons.json",&json!({"warmup_fixtures":4,"measured_fixtures_per_size":20,"cached_updates_per_fixture":64,"native_icon_pixels":[SIZE,SIZE],"boundary":"icons update_root only, including cache admission, Job submission/consumption and Image projection; excludes layout/render/presentation; directory snapshots prepared before timing","scenarios":reports})).unwrap();
    }
}
