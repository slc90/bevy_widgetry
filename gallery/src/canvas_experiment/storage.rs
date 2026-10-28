use super::*;
use bevy_widgetry::canvas::{CanvasGeometry, CanvasPaint};
use serde::Serialize;
use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    version: u32,
    session: String,
    width: u32,
    height: u32,
    next_stage: u64,
    shapes: Vec<drawing::ShapeRecord>,
    retired_ids: Vec<String>,
}

pub(super) fn save(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Identify = params(value)?;
    world.resource_scope(|world, mut sessions: Mut<Sessions>| {
        let session = sessions
            .0
            .get_mut(&request.session)
            .ok_or_else(|| invalid_params("Unknown session"))?;
        persist(world, &request.session, session);
        describe(world, &request.session, session)
    })
}

pub(super) fn persist(world: &mut World, id: &str, session: &mut Session) {
    session.dirty = true;
    let result = snapshot(world, id, session).and_then(|archive| {
        let bytes = serde_json::to_vec_pretty(&archive).map_err(BrpError::resource_error)?;
        replace(&session.directory.join("document.json"), &bytes).map_err(BrpError::resource_error)
    });
    match result {
        Ok(()) => {
            session.dirty = false;
            session.save_error = None;
        }
        Err(error) => {
            warn!(session = id, path = %session.directory.display(), error = %error.message, "绘画已生效但工作存档保存失败");
            session.save_error = Some(error.message);
        }
    }
}

fn snapshot(world: &mut World, id: &str, session: &Session) -> Result<Archive, BrpError> {
    let reverse: BTreeMap<_, _> = session
        .shapes
        .iter()
        .map(|(id, entity)| (*entity, id))
        .collect();
    let order =
        CanvasDocument::shape_order(world, session.document).map_err(BrpError::resource_error)?;
    let mut active = BTreeSet::new();
    let mut shapes = Vec::with_capacity(order.len());
    for entity in order {
        let id = reverse.get(&entity).ok_or_else(|| {
            BrpError::resource_error("Document contains a shape without a stable ID")
        })?;
        active.insert(id.as_str());
        shapes.push(drawing::record(world, session, id)?);
    }
    Ok(Archive {
        version: 1,
        session: id.into(),
        width: session.size.x,
        height: session.size.y,
        next_stage: session.next_stage,
        shapes,
        retired_ids: session
            .shapes
            .keys()
            .filter(|id| !active.contains(id.as_str()))
            .cloned()
            .collect(),
    })
}

fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(format!(".tmp-{}-{timestamp}", std::process::id()));
    let temporary = PathBuf::from(temporary);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    let result = written.and_then(|()| std::fs::rename(&temporary, path));
    if result.is_err()
        && temporary.exists()
        && let Err(error) = std::fs::remove_file(&temporary)
    {
        warn!(%error, path = %temporary.display(), "绘画存档临时文件清理失败");
    }
    result
}

pub(super) fn load(world: &mut World, id: &str) -> Result<Session, BrpError> {
    if !id.starts_with("painting-") || !drawing::valid_id(id) {
        return Err(invalid_params("Invalid painting session ID"));
    }
    load_from(world, id, exports_root().join(id))
}

fn load_from(world: &mut World, id: &str, directory: PathBuf) -> Result<Session, BrpError> {
    let bytes = std::fs::read(directory.join("document.json")).map_err(BrpError::resource_error)?;
    let archive: Archive = serde_json::from_slice(&bytes).map_err(invalid_params)?;
    if archive.version != 1
        || archive.session != id
        || archive.width == 0
        || archive.height == 0
        || archive.width > 16_777_216
        || archive.height > 16_777_216
        || archive.next_stage == 0
    {
        return Err(invalid_params(
            "Invalid painting archive identity, dimensions or version",
        ));
    }
    let mut ids = BTreeSet::new();
    for id in archive
        .shapes
        .iter()
        .map(|shape| &shape.id)
        .chain(&archive.retired_ids)
    {
        if !drawing::valid_id(id) || !ids.insert(id.clone()) {
            return Err(invalid_params("Invalid or duplicate archived shape ID"));
        }
    }
    let next_stage = super::stages::next_number(&directory, archive.next_stage)
        .map_err(BrpError::resource_error)?;
    let document = world
        .spawn((CanvasDocument, Name::new(format!("PaintingDocument-{id}"))))
        .id();
    let mut session = Session {
        document,
        size: UVec2::new(archive.width, archive.height),
        directory,
        window: None,
        shapes: archive
            .retired_ids
            .into_iter()
            .map(|id| (id, Entity::PLACEHOLDER))
            .collect(),
        next_stage,
        dirty: false,
        save_error: None,
    };
    let rebuilt = (|| {
        for shape in archive.shapes {
            let geometry = drawing::decode::<CanvasGeometry>(world, shape.geometry)?;
            let paint = drawing::decode::<CanvasPaint>(world, shape.paint)?;
            let transform = drawing::decode::<Transform>(world, shape.transform)?;
            let entity = drawing::create_shape(world, document, geometry, paint, transform)
                .map_err(BrpError::resource_error)?;
            session.shapes.insert(shape.id, entity);
        }
        Ok(())
    })();
    if let Err(error) = rebuilt {
        world.despawn(document);
        return Err(error);
    }
    Ok(session)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn atomic_replace_preserves_previous_file_on_windows_sharing_failure() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.json");
        replace(&path, b"old").unwrap();
        replace(&path, b"new").unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(replace(&path, b"broken").is_err());
        drop(locked);
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn restore_keeps_order_identity_and_rejects_corruption_without_leaking_entities() {
        let mut app = App::new();
        app.register_type::<CanvasGeometry>()
            .register_type::<CanvasPaint>()
            .register_type::<Transform>();
        let world = app.world_mut();
        let directory = tempfile::tempdir().unwrap();
        let document = world.spawn(CanvasDocument).id();
        let first = CanvasDocument::create_ellipse(
            world,
            document,
            bevy_widgetry::canvas::CanvasEllipse {
                radii: Vec2::new(4.0, 7.0),
            },
            Transform::from_rotation(Quat::from_rotation_z(0.4))
                .with_scale(Vec3::new(-2.0, 0.5, 1.0)),
            CanvasPaint::default(),
        )
        .unwrap();
        let second = CanvasDocument::create_path(
            world,
            document,
            bevy_widgetry::canvas::CanvasPath {
                commands: vec![
                    bevy_widgetry::canvas::CanvasPathCommand::MoveTo(Vec2::ZERO),
                    bevy_widgetry::canvas::CanvasPathCommand::LineTo(Vec2::splat(5.0)),
                ],
            },
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        let mut session = Session {
            document,
            size: UVec2::new(100, 200),
            directory: directory.path().into(),
            window: None,
            shapes: BTreeMap::from([
                ("first".into(), first),
                ("second".into(), second),
                ("deleted".into(), Entity::PLACEHOLDER),
            ]),
            next_stage: 7,
            dirty: true,
            save_error: None,
        };
        CanvasDocument::reorder_shape(
            world,
            document,
            second,
            bevy_widgetry::canvas::CanvasOrder::Back,
        )
        .unwrap();
        persist(world, "painting-test", &mut session);
        assert!(!session.dirty);
        let before =
            serde_json::to_value(snapshot(world, "painting-test", &session).unwrap()).unwrap();
        let restored = load_from(world, "painting-test", directory.path().into()).unwrap();
        assert_ne!(restored.document, document);
        assert_ne!(restored.shapes["first"], first);
        assert_eq!(
            serde_json::to_value(snapshot(world, "painting-test", &restored).unwrap()).unwrap(),
            before
        );
        let count = world.entities().len();
        let mut damaged = before;
        damaged["shapes"][1]["geometry"] = json!({"Ellipse":{"radii":[-1,2]}});
        replace(
            &directory.path().join("document.json"),
            &serde_json::to_vec(&damaged).unwrap(),
        )
        .unwrap();
        assert!(load_from(world, "painting-test", directory.path().into()).is_err());
        assert_eq!(world.entities().len(), count);
        assert_eq!(
            CanvasDocument::shape_order(world, document).unwrap(),
            vec![second, first]
        );
        assert_eq!(
            std::fs::read(directory.path().join("document.json")).unwrap(),
            serde_json::to_vec(&damaged).unwrap()
        );
    }
}
