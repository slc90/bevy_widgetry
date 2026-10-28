use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::window::{RequestRedraw, WindowCloseRequested, WindowRef};
use bevy_remote::{BrpError, BrpResult, RemoteMethodSystemId, RemoteMethods};
use bevy_widgetry::canvas::{CanvasDocument, CanvasView};
use bevy_widgetry::scene::spawn_scene;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

mod drawing;
mod storage;

#[derive(Resource, Default)]
struct Sessions(BTreeMap<String, Session>);

struct Session {
    document: Entity,
    size: UVec2,
    directory: PathBuf,
    window: Option<SessionWindow>,
    shapes: BTreeMap<String, Entity>,
    next_stage: u64,
    dirty: bool,
    save_error: Option<String>,
}

#[derive(Clone, Copy)]
struct SessionWindow {
    native: Entity,
    camera: Entity,
    view: Entity,
}

#[derive(Component)]
struct ExperimentView {
    size: UVec2,
    window: Entity,
    available: Vec2,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Create {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identify {
    session: String,
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<Sessions>()
        .add_systems(Update, close_windows)
        .add_systems(
            PostUpdate,
            fit_views.before(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        );
    if !app.world().contains_resource::<RemoteMethods>() {
        return;
    }
    let create = app.world_mut().register_system(create);
    let open = app.world_mut().register_system(open);
    let query = app.world_mut().register_system(query);
    let batch = app.world_mut().register_system(drawing::batch);
    let shapes = app.world_mut().register_system(drawing::query);
    let save = app.world_mut().register_system(storage::save);
    let restore = app.world_mut().register_system(restore);
    let mut methods = app.world_mut().resource_mut::<RemoteMethods>();
    for (name, handler) in [
        ("gallery/canvas_create", create),
        ("gallery/canvas_open", open),
        ("gallery/canvas_session", query),
        ("gallery/canvas_batch", batch),
        ("gallery/canvas_shapes", shapes),
        ("gallery/canvas_save", save),
        ("gallery/canvas_restore", restore),
    ] {
        methods.insert(name, RemoteMethodSystemId::Instant(handler));
    }
}

fn params<T: serde::de::DeserializeOwned>(value: Option<Value>) -> Result<T, BrpError> {
    serde_json::from_value(value.unwrap_or(Value::Null)).map_err(invalid_params)
}

fn invalid_params(error: impl ToString) -> BrpError {
    BrpError {
        code: -32602,
        message: error.to_string(),
        data: None,
    }
}

fn create(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Create = params(value)?;
    // f32 represents every integer through 2^24; larger dimensions cannot retain pixel coordinates.
    if request.width == 0
        || request.height == 0
        || request.width > 16_777_216
        || request.height > 16_777_216
    {
        return Err(invalid_params("Dimensions must be in 1..=16777216"));
    }
    let root = exports_root();
    std::fs::create_dir_all(&root).map_err(BrpError::resource_error)?;
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let (id, directory) = (0u64..)
        .find_map(|suffix| {
            let id = format!("painting-{timestamp}-{suffix}");
            let directory = root.join(&id);
            match std::fs::create_dir(&directory) {
                Ok(()) => Some(Ok((id, directory))),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                Err(error) => Some(Err(error)),
            }
        })
        .ok_or_else(|| BrpError::resource_error("No free session directory"))?
        .map_err(BrpError::resource_error)?;
    let document = world
        .spawn((CanvasDocument, Name::new(format!("PaintingDocument-{id}"))))
        .id();
    let mut session = Session {
        document,
        size: UVec2::new(request.width, request.height),
        directory,
        window: None,
        shapes: BTreeMap::new(),
        next_stage: 1,
        dirty: true,
        save_error: None,
    };
    if let Err(error) = open_window(world, &id, &mut session) {
        world.despawn(document);
        if let Err(cleanup) = std::fs::remove_dir(&session.directory) {
            warn!(%cleanup, path = %session.directory.display(), "绘画会话创建失败后目录清理失败");
        }
        return Err(BrpError::resource_error(error));
    }
    storage::persist(world, &id, &mut session);
    let response = describe(world, &id, &session)?;
    world.resource_mut::<Sessions>().0.insert(id, session);
    Ok(response)
}

fn open(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Identify = params(value)?;
    world.resource_scope(|world, mut sessions: Mut<Sessions>| {
        if !sessions.0.contains_key(&request.session) {
            let mut session = storage::load(world, &request.session)?;
            if let Err(error) = open_window(world, &request.session, &mut session) {
                world.despawn(session.document);
                return Err(BrpError::resource_error(error));
            }
            sessions.0.insert(request.session.clone(), session);
        }
        let session = sessions
            .0
            .get_mut(&request.session)
            .ok_or_else(|| invalid_params("Unknown session"))?;
        open_window(world, &request.session, session).map_err(BrpError::resource_error)?;
        describe(world, &request.session, session)
    })
}

fn exports_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("exports")
}

fn restore(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Identify = params(value)?;
    let mut loaded = storage::load(world, &request.session)?;
    if let Err(error) = open_window(world, &request.session, &mut loaded) {
        world.despawn(loaded.document);
        return Err(BrpError::resource_error(error));
    }
    let response = describe(world, &request.session, &loaded)?;
    if let Some(previous) = world
        .resource_mut::<Sessions>()
        .0
        .insert(request.session.clone(), loaded)
    {
        if let Some(window) = previous.window {
            destroy_window(world, window);
        }
        world.despawn(previous.document);
    }
    info!(session = request.session, "通过 BRP 恢复绘画工作存档");
    Ok(response)
}

fn query(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Identify = params(value)?;
    world.resource_scope(|world, sessions: Mut<Sessions>| {
        let session = sessions
            .0
            .get(&request.session)
            .ok_or_else(|| invalid_params("Unknown session"))?;
        describe(world, &request.session, session)
    })
}

fn describe(world: &mut World, id: &str, session: &Session) -> BrpResult {
    let window = session
        .window
        .filter(|window| world.get::<Window>(window.native).is_some());
    let view = window.map(|window| window.view);
    let zoom = view
        .map(|view| CanvasView::zoom(world, view))
        .transpose()
        .map_err(BrpError::resource_error)?;
    let center = view
        .map(|view| CanvasView::center(world, view))
        .transpose()
        .map_err(BrpError::resource_error)?;
    Ok(json!({
        "session": id, "width": session.size.x, "height": session.size.y,
        "region": [0, 0, session.size.x, session.size.y],
        "directory": session.directory, "document": session.document.to_bits(),
        "window": window.map(|window| window.native.to_bits()),
        "camera": window.map(|window| window.camera.to_bits()),
        "view": view.map(Entity::to_bits), "zoom": zoom,
        "center": center.map(|center| [center.x, center.y]),
        "shape_count": CanvasDocument::shape_order(world, session.document).map_err(BrpError::resource_error)?.len(),
        "fit_limited": zoom.is_some_and(|zoom| zoom == 0.1 || zoom == 10.0),
        "persistence": {"saved": !session.dirty, "error": session.save_error},
        "next_stage": session.next_stage,
    }))
}

fn open_window(world: &mut World, id: &str, session: &mut Session) -> Result {
    if let Some(window) = session.window {
        if world.get::<Window>(window.native).is_some() {
            return Ok(());
        }
        destroy_window(world, window);
        session.window = None;
    }
    let native = world
        .spawn((
            Window {
                title: format!("Canvas — {id}"),
                resolution: (1000, 760).into(),
                ..default()
            },
            Name::new(format!("PaintingWindow-{id}")),
        ))
        .id();
    let camera = world
        .spawn((Camera2d, RenderTarget::Window(WindowRef::Entity(native))))
        .id();
    let document = session.document;
    let size = session.size;
    let scene = bsn! {
        @CanvasView { @document: {Some(document)}, @center: {size.as_vec2() * 0.5} }
        template(move |_| Ok(ExperimentView { size, window: native, available: Vec2::ZERO }))
        Name({format!("PaintingView-{id}")})
        UiTargetCamera(camera)
        Node { width: percent(100), height: percent(100) }
        BackgroundColor(Color::srgb(0.12, 0.12, 0.12))
    };
    let view = match spawn_scene(world, scene) {
        Ok(view) => view,
        Err(error) => {
            world.despawn(camera);
            world.despawn(native);
            return Err(BevyError::error(error));
        }
    };
    session.window = Some(SessionWindow {
        native,
        camera,
        view,
    });
    world.write_message(RequestRedraw);
    info!(session = id, ?native, "通过 BRP 打开独立绘画窗口");
    Ok(())
}

fn destroy_window(world: &mut World, window: SessionWindow) {
    for entity in [window.view, window.camera, window.native] {
        if world.entities().contains(entity) {
            world.despawn(entity);
        }
    }
}

fn close_windows(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<WindowCloseRequested>>,
) {
    let requested: Vec<_> = cursor
        .read(world.resource::<Messages<WindowCloseRequested>>())
        .map(|request| request.window)
        .collect();
    world.resource_scope(|world, mut sessions: Mut<Sessions>| {
        for (id, session) in &mut sessions.0 {
            if let Some(window) = session.window
                && (requested.contains(&window.native)
                    || world.get::<Window>(window.native).is_none())
            {
                destroy_window(world, window);
                session.window = None;
                info!(session = id, "关闭独立绘画窗口并保留会话");
            }
        }
    });
}

fn fit_views(world: &mut World) -> Result {
    let scale = world.resource::<UiScale>().0;
    let changed: Vec<_> = world
        .query::<(Entity, &ExperimentView)>()
        .iter(world)
        .filter_map(|(entity, view)| {
            let window = world.get::<Window>(view.window)?;
            let available = Vec2::new(window.width(), window.height()) / scale;
            (available != view.available).then_some((entity, available, view.size))
        })
        .collect();
    for (view, available, size) in changed {
        if available.min_element() > 0.0 {
            let zoom = (available / size.as_vec2()).min_element().clamp(0.1, 10.0);
            CanvasView::set_zoom_in_world(world, view, zoom)?;
            CanvasView::set_center_in_world(world, view, size.as_vec2() * 0.5)?;
            if let Some(mut state) = world.get_mut::<ExperimentView>(view) {
                state.available = available;
            }
            world.write_message(RequestRedraw);
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn resize_commits_navigation_before_canvas_preparation() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.init_resource::<UiScale>()
            .add_message::<RequestRedraw>();
        app.add_systems(
            PostUpdate,
            fit_views.before(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        );
        let world = app.world_mut();
        let window = world
            .spawn(Window {
                resolution: (800, 600).into(),
                ..default()
            })
            .id();
        let view = spawn_scene(world, bsn! {
            @CanvasView { @center: Vec2::new(-90.0, -70.0), @zoom: 3.0 }
            template(move |_| Ok(ExperimentView { size: UVec2::new(1600, 1200), window, available: Vec2::ZERO }))
        }).unwrap();
        app.add_systems(
            PostUpdate,
            (move |world: &mut World| {
                assert_eq!(
                    CanvasView::center(world, view).unwrap(),
                    Vec2::new(800.0, 600.0)
                );
                assert_eq!(CanvasView::zoom(world, view).unwrap(), 0.5);
            })
            .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Build),
        );
        app.world_mut().run_schedule(PostUpdate);
        CanvasView::set_center_in_world(app.world_mut(), view, Vec2::ZERO).unwrap();
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(1800.0, 1200.0);
        app.world_mut().resource_mut::<UiScale>().0 = 2.0;
        app.world_mut().run_schedule(PostUpdate);
    }
}
