use super::*;
use bevy_widgetry::canvas::{
    CanvasExportError, CanvasGeometry, CanvasPaint, CanvasPngExport, CanvasPngExportFinished,
    CanvasPngOptions,
};
use bevy_widgetry::job::{JobAccessError, JobId, JobManager};
use std::io;
use std::path::Path;

#[derive(Resource, Default)]
struct Stages(BTreeMap<(String, u64), Stage>);

struct Stage {
    document: Entity,
    region: Rect,
    files: Vec<StageFile>,
}

struct StageFile {
    path: PathBuf,
    scale: Option<u32>,
    size: UVec2,
    state: FileState,
}

enum FileState {
    Queued,
    Exporting {
        request: Entity,
        job: JobId,
        temporary: PathBuf,
    },
    Succeeded,
    Failed(String),
    Canceled,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentifyStage {
    session: String,
    stage: u64,
}

impl Stage {
    fn pending(&self) -> bool {
        self.files
            .iter()
            .any(|file| matches!(file.state, FileState::Queued | FileState::Exporting { .. }))
    }

    fn describe(&self, session: &str, number: u64) -> Value {
        let succeeded = self
            .files
            .iter()
            .filter(|file| matches!(file.state, FileState::Succeeded))
            .count();
        let status = if self.pending() {
            "exporting"
        } else if succeeded == self.files.len() {
            "succeeded"
        } else if self
            .files
            .iter()
            .all(|file| !matches!(file.state, FileState::Failed(_)))
        {
            "canceled"
        } else if succeeded != 0 {
            "partial_failure"
        } else {
            "failed"
        };
        let files: Vec<_> = self
            .files
            .iter()
            .map(|file| {
                let (status, error, request, job) = match &file.state {
                    FileState::Queued => ("queued", None, None, None),
                    FileState::Exporting { request, job, .. } => {
                        ("exporting", None, Some(request.to_bits()), Some(job.get()))
                    }
                    FileState::Succeeded => ("succeeded", None, None, None),
                    FileState::Failed(error) => ("failed", Some(error.as_str()), None, None),
                    FileState::Canceled => ("canceled", None, None, None),
                };
                json!({"format": if file.scale.is_some() {"png"} else {"svg"}, "scale": file.scale,
                "path": file.path, "width":file.size.x,"height":file.size.y,
                "status":status,"error":error,"request":request,"job_id":job})
            })
            .collect();
        json!({"session":session,"stage":number,"accepted":true,"status":status,"files":files})
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Stages>().add_observer(completed);
}

pub(crate) fn pending(world: &World) -> bool {
    world.get_resource::<Stages>().is_some_and(|stages| {
        stages
            .0
            .values()
            .any(|stage| stage.pending() || world.get::<CanvasDocument>(stage.document).is_some())
    })
}

pub(super) fn export(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Identify = params(value)?;
    if pending(world) {
        return Err(BrpError::resource_error(
            "A stage export is already running; query it and retry after completion",
        ));
    }
    world.resource_scope(|world, mut sessions: Mut<Sessions>| {
        let session = sessions.0.get_mut(&request.session).ok_or_else(|| invalid_params("Unknown session"))?;
        if CanvasDocument::shape_order(world, session.document).map_err(BrpError::resource_error)?.is_empty() {
            return Err(BrpError::resource_error(CanvasExportError::EmptyDocument));
        }
        let number = reserve(&session.directory, &mut session.next_stage).map_err(BrpError::resource_error)?;
        storage::persist(world, &request.session, session);
        let directory = session.directory.join("stages");
        let region = Rect::from_corners(Vec2::ZERO, session.size.as_vec2());
        let document = freeze(world, session.document).map_err(BrpError::resource_error)?;
        let mut files = Vec::with_capacity(4);
        for scale in [None, Some(1), Some(2), Some(4)] {
            let name = match scale {
                None => format!("stage_{number:04}.svg"),
                Some(scale) => format!("stage_{number:04}_{scale}x.png"),
            };
            let path = directory.join(name);
            let size = session.size.checked_mul(UVec2::splat(scale.unwrap_or(1)));
            let state = match size {
                Some(_) if scale.is_none() => start(world, document, region, scale, &path),
                Some(_) => FileState::Queued,
                None => FileState::Failed("Output dimensions overflow".into()),
            };
            files.push(StageFile { path, scale, size: size.unwrap_or(UVec2::ZERO), state });
        }
        let mut stage = Stage {document, region, files};
        advance(world, &mut stage, &request.session, number);
        let mut response = stage.describe(&request.session, number);
        response["persistence"] = json!({"saved":!session.dirty,"error":session.save_error});
        info!(session=request.session, stage=number, status=%response["status"], "通过 BRP 发起绘画阶段导出");
        world.resource_mut::<Stages>().0.insert((request.session, number), stage);
        world.write_message(RequestRedraw);
        Ok(response)
    })
}

fn freeze(world: &mut World, source: Entity) -> Result<Entity> {
    let order = CanvasDocument::shape_order(world, source)?;
    let mut shapes = Vec::with_capacity(order.len());
    for shape in order {
        let geometry = world
            .get::<CanvasGeometry>(shape)
            .cloned()
            .ok_or_else(|| BevyError::error("Stage shape geometry missing"))?;
        let paint = world
            .get::<CanvasPaint>(shape)
            .copied()
            .ok_or_else(|| BevyError::error("Stage shape paint missing"))?;
        let transform = world
            .get::<Transform>(shape)
            .copied()
            .ok_or_else(|| BevyError::error("Stage shape transform missing"))?;
        geometry.validate(&transform, &paint)?;
        shapes.push((geometry, paint, transform));
    }
    let document = world
        .spawn((CanvasDocument, Name::new("PaintingStageSnapshot")))
        .id();
    for (geometry, paint, transform) in shapes {
        if let Err(error) = drawing::create_shape(world, document, geometry, paint, transform) {
            world.despawn(document);
            return Err(error);
        }
    }
    Ok(document)
}

fn advance(world: &mut World, stage: &mut Stage, session: &str, number: u64) {
    if stage
        .files
        .iter()
        .any(|file| matches!(file.state, FileState::Exporting { .. }))
    {
        return;
    }
    for file in &mut stage.files {
        if matches!(file.state, FileState::Queued) {
            file.state = start(world, stage.document, stage.region, file.scale, &file.path);
            if matches!(file.state, FileState::Exporting { .. }) {
                return;
            }
        }
    }
    if world.get_entity(stage.document).is_ok() {
        world.despawn(stage.document);
        info!(session,stage=number,status=%stage.describe(session,number)["status"],"绘画阶段导出已收尾");
    }
}

fn start(
    world: &mut World,
    document: Entity,
    region: Rect,
    scale: Option<u32>,
    path: &Path,
) -> FileState {
    let result = (|| {
        let temporary = temporary(path).map_err(|error| error.to_string())?;
        let result = match scale {
            None => CanvasDocument::export_svg(world, document, &temporary, Some(region))
                .map_err(|error| error.to_string())
                .and_then(|()| publish(&temporary, path).map_err(|error| error.to_string()))
                .map(|()| FileState::Succeeded),
            Some(scale) => CanvasDocument::export_png(
                world,
                document,
                &temporary,
                CanvasPngOptions {
                    region: Some(region),
                    scale: scale as f32,
                },
            )
            .map_err(|error| error.to_string())
            .and_then(|entity| {
                let job = world
                    .get::<CanvasPngExport>(entity)
                    .ok_or_else(|| "PNG request has no export identity".to_owned())?
                    .job_id();
                Ok(FileState::Exporting {
                    request: entity,
                    job,
                    temporary: temporary.clone(),
                })
            }),
        };
        if !matches!(result, Ok(FileState::Exporting { .. })) {
            cleanup(&temporary);
        }
        result
    })();
    match result {
        Ok(state) => state,
        Err(error) => {
            warn!(?path,%error,"绘画阶段文件导出请求失败");
            FileState::Failed(error)
        }
    }
}

pub(super) fn query(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: IdentifyStage = params(value)?;
    let stage = world.resource::<Stages>().0.get(&(request.session.clone(), request.stage))
        .ok_or_else(|| invalid_params("Stage is not tracked in this process; historical files remain in the session stages directory"))?;
    Ok(stage.describe(&request.session, request.stage))
}

pub(super) fn cancel(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: IdentifyStage = params(value)?;
    let manager = world.resource::<JobManager>().clone();
    let response = world.resource_scope(|world, mut stages: Mut<Stages>| {
        let stage = stages
            .0
            .get_mut(&(request.session.clone(), request.stage))
            .ok_or_else(|| invalid_params("Unknown stage"))?;
        let pending = request_cancel(stage, &manager).map_err(BrpError::resource_error)?;
        advance(world, stage, &request.session, request.stage);
        let mut response = stage.describe(&request.session, request.stage);
        response["cancel_requested"] = json!(pending);
        Ok(response)
    });
    info!(
        session = request.session,
        stage = request.stage,
        "通过 BRP 请求取消绘画阶段导出"
    );
    response
}

fn request_cancel(stage: &mut Stage, manager: &JobManager) -> Result<bool, JobAccessError> {
    let pending = stage.pending();
    for file in &mut stage.files {
        match &file.state {
            FileState::Exporting { job, .. } => {
                manager.cancel(*job)?;
            }
            FileState::Queued => file.state = FileState::Canceled,
            _ => {}
        }
    }
    Ok(pending)
}

pub(crate) fn cancel_all(world: &mut World) -> Result {
    if !world.contains_resource::<Stages>() {
        return Ok(());
    }
    let manager = world.resource::<JobManager>().clone();
    world.resource_scope(|world, mut stages: Mut<Stages>| {
        for ((session, number), stage) in &mut stages.0 {
            let canceled = request_cancel(stage, &manager).map_err(|error| {
                error!(session,stage=number,%error,"Gallery 退出无法取消绘画阶段任务");
                BevyError::error(error.to_string())
            })?;
            if canceled {
                info!(session, stage = number, "Gallery 退出取消绘画阶段导出");
                advance(world, stage, session, *number);
            }
        }
        Ok(())
    })
}

fn completed(
    event: On<CanvasPngExportFinished>,
    mut stages: ResMut<Stages>,
    mut commands: Commands,
) {
    for ((session, number), stage) in &mut stages.0 {
        if stage.document != event.document {
            continue;
        }
        for file in &mut stage.files {
            let FileState::Exporting {
                request,
                job,
                temporary,
            } = &file.state
            else {
                continue;
            };
            if *request != event.entity || *job != event.job_id || *temporary != event.path {
                continue;
            }
            let state = match &event.result {
                Ok(size) if *size == file.size => match publish(temporary, &file.path) {
                    Ok(()) => FileState::Succeeded,
                    Err(error) => FileState::Failed(error.to_string()),
                },
                Ok(_) => FileState::Failed("PNG output dimensions do not match the stage".into()),
                Err(CanvasExportError::Canceled) => FileState::Canceled,
                Err(error) => FileState::Failed(error.to_string()),
            };
            cleanup(temporary);
            if let FileState::Failed(error) = &state {
                warn!(session, stage=number, path=%file.path.display(), %error,"绘画阶段 PNG 导出失败");
            }
            file.state = state;
            let session = session.clone();
            let number = *number;
            commands.queue(move |world: &mut World| {
                world.resource_scope(|world, mut stages: Mut<Stages>| {
                    if let Some(stage) = stages.0.get_mut(&(session.clone(), number)) {
                        advance(world, stage, &session, number);
                    }
                })
            });
            return;
        }
    }
}

pub(super) fn next_number(directory: &Path, mut next: u64) -> io::Result<u64> {
    let entries = match std::fs::read_dir(directory.join("stages")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(next),
        Err(error) => return Err(error),
    };
    for entry in entries {
        let name = entry?.file_name();
        let name = name.to_string_lossy();
        if let Some(rest) = name.strip_prefix("stage_") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(number) = digits.parse::<u64>() {
                next = next.max(
                    number
                        .checked_add(1)
                        .ok_or_else(|| io::Error::other("Stage number exhausted"))?,
                );
            }
        }
    }
    Ok(next)
}

fn reserve(directory: &Path, next: &mut u64) -> io::Result<u64> {
    std::fs::create_dir_all(directory.join("stages"))?;
    *next = next_number(directory, *next)?;
    loop {
        let number = *next;
        *next = number
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Stage number exhausted"))?;
        let path = directory
            .join("stages")
            .join(format!("stage_{number:04}.reserve"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => {
                file.sync_all()?;
                return Ok(number);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

fn temporary(path: &Path) -> io::Result<PathBuf> {
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".tmp-{}-{timestamp}", std::process::id()));
    let temporary = PathBuf::from(name);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    Ok(temporary)
}

fn publish(temporary: &Path, path: &Path) -> io::Result<()> {
    std::fs::OpenOptions::new()
        .write(true)
        .open(temporary)?
        .sync_all()?;
    // Both paths are on the same volume; hard_link atomically refuses an existing destination.
    std::fs::hard_link(temporary, path)
}

fn cleanup(path: &Path) {
    if let Err(error) = std::fs::remove_file(path)
        && error.kind() != io::ErrorKind::NotFound
    {
        warn!(%error, path=%path.display(),"绘画阶段临时文件清理失败");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry::canvas::{CanvasGeometry, CanvasPaint, CanvasRectangle};

    #[test]
    fn frozen_stage_keeps_geometry_paint_and_order_after_source_edits() {
        let mut world = World::new();
        let source = world.spawn(CanvasDocument).id();
        let rectangle = CanvasDocument::create_rectangle(
            &mut world,
            source,
            CanvasRectangle {
                size: Vec2::splat(20.0),
                corner_radius: 0.0,
            },
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        let ellipse = CanvasDocument::create_ellipse(
            &mut world,
            source,
            bevy_widgetry::canvas::CanvasEllipse {
                radii: Vec2::splat(5.0),
            },
            Transform::from_xyz(10.0, 20.0, 0.0),
            CanvasPaint::default(),
        )
        .unwrap();
        CanvasDocument::reorder_shape(
            &mut world,
            source,
            ellipse,
            bevy_widgetry::canvas::CanvasOrder::Back,
        )
        .unwrap();
        let snapshot = freeze(&mut world, source).unwrap();
        CanvasDocument::update_rectangle(
            &mut world,
            source,
            rectangle,
            CanvasRectangle {
                size: Vec2::splat(50.0),
                corner_radius: 1.0,
            },
            Transform::from_xyz(100.0, 100.0, 0.0),
            CanvasPaint {
                fill: Some(Color::BLACK),
                ..default()
            },
        )
        .unwrap();
        CanvasDocument::remove_shape(&mut world, source, ellipse).unwrap();
        let shapes = CanvasDocument::shape_order(&mut world, snapshot).unwrap();
        assert_eq!(shapes.len(), 2);
        assert!(matches!(
            world.get::<CanvasGeometry>(shapes[0]),
            Some(CanvasGeometry::Ellipse(_))
        ));
        assert_eq!(
            world.get::<Transform>(shapes[0]).unwrap().translation,
            Vec3::new(10.0, 20.0, 0.0)
        );
        assert!(
            matches!(world.get::<CanvasGeometry>(shapes[1]),Some(CanvasGeometry::Rectangle(rect)) if rect.size == Vec2::splat(20.0))
        );
        assert_eq!(
            world.get::<CanvasPaint>(shapes[1]).unwrap().fill,
            CanvasPaint::default().fill
        );
        world.despawn(snapshot);
        for shape in shapes {
            assert!(world.get_entity(shape).is_err());
        }
    }

    #[test]
    #[ignore = "Manual optimized CPU snapshot benchmark; excludes GPU, files and BRP transport"]
    fn benchmark_stage_freeze() {
        use std::fmt::Write;
        use std::time::Instant;
        let artifact = bevy_widgetry_test_utils::benchmark::artifact::Artifact::new(
            "canvas-stage-freeze", "optimized Cargo test profile; public CanvasDocument snapshot cloning and destruction; 20 warmups plus 200 samples; no render/files/BRP transport"
        ).unwrap();
        let mut csv =
            String::from("shapes,sample,freeze_us,release_us,entities_before,entities_after\n");
        for count in [100, 1000] {
            let mut world = World::new();
            let document = world.spawn(CanvasDocument).id();
            for index in 0..count {
                let geometry = match index % 3 {
                    0 => CanvasGeometry::Rectangle(CanvasRectangle {
                        size: Vec2::splat(20.0),
                        corner_radius: 2.0,
                    }),
                    1 => CanvasGeometry::Ellipse(bevy_widgetry::canvas::CanvasEllipse {
                        radii: Vec2::new(10.0, 20.0),
                    }),
                    _ => CanvasGeometry::Path(bevy_widgetry::canvas::CanvasPath {
                        commands: vec![
                            bevy_widgetry::canvas::CanvasPathCommand::MoveTo(Vec2::ZERO),
                            bevy_widgetry::canvas::CanvasPathCommand::CubicTo {
                                control1: Vec2::X * 10.0,
                                control2: Vec2::Y * 10.0,
                                to: Vec2::splat(20.0),
                            },
                            bevy_widgetry::canvas::CanvasPathCommand::Close,
                        ],
                    }),
                };
                drawing::create_shape(
                    &mut world,
                    document,
                    geometry,
                    CanvasPaint::default(),
                    Transform::from_xyz(index as f32, 0.0, 0.0),
                )
                .unwrap();
            }
            let baseline = world.entities().count_spawned();
            for sample in 0..220 {
                let started = Instant::now();
                let snapshot = std::hint::black_box(freeze(&mut world, document).unwrap());
                let frozen = started.elapsed().as_nanos() as f64 / 1000.0;
                let started = Instant::now();
                world.despawn(snapshot);
                let released = started.elapsed().as_nanos() as f64 / 1000.0;
                assert_eq!(world.entities().count_spawned(), baseline);
                if sample >= 20 {
                    writeln!(
                        csv,
                        "{count},{},{frozen},{released},{baseline},{}",
                        sample - 20,
                        world.entities().count_spawned()
                    )
                    .unwrap();
                }
            }
        }
        std::fs::write(artifact.directory.join("samples.csv"), csv).unwrap();
        println!(
            "Stage freeze CPU benchmark: {}",
            artifact.directory.display()
        );
    }

    #[test]
    fn reservations_survive_stale_counters_partial_files_and_other_reservers() {
        let directory = tempfile::tempdir().unwrap();
        let mut next = 1;
        assert_eq!(reserve(directory.path(), &mut next).unwrap(), 1);
        std::fs::write(
            directory.path().join("stages/stage_0007_4x.png"),
            b"partial",
        )
        .unwrap();
        assert_eq!(reserve(directory.path(), &mut next).unwrap(), 8);
        let mut restored = 1;
        assert_eq!(reserve(directory.path(), &mut restored).unwrap(), 9);
        assert_eq!(next_number(directory.path(), 20).unwrap(), 20);
        assert_eq!(
            std::fs::read(directory.path().join("stages/stage_0007_4x.png")).unwrap(),
            b"partial"
        );
    }

    #[test]
    fn publication_refuses_existing_history_and_retains_both_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("stage_0001.svg");
        let first = temporary(&path).unwrap();
        std::fs::write(&first, b"original").unwrap();
        publish(&first, &path).unwrap();
        cleanup(&first);
        let second = temporary(&path).unwrap();
        std::fs::write(&second, b"replacement").unwrap();
        assert!(publish(&second, &path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        assert_eq!(std::fs::read(&second).unwrap(), b"replacement");
    }

    #[test]
    fn global_shutdown_cancels_unsubmitted_files_and_releases_the_frozen_document() {
        let mut app = App::new();
        app.insert_resource(JobManager::default());
        install(&mut app);
        let document = app.world_mut().spawn(CanvasDocument).id();
        app.world_mut().resource_mut::<Stages>().0.insert(
            ("painting-test".into(), 1),
            Stage {
                document,
                region: Rect::from_corners(Vec2::ZERO, Vec2::splat(10.0)),
                files: [1, 2, 4]
                    .into_iter()
                    .map(|scale| StageFile {
                        path: PathBuf::from(format!("unused-{scale}.png")),
                        scale: Some(scale),
                        size: UVec2::splat(10 * scale),
                        state: FileState::Queued,
                    })
                    .collect(),
            },
        );
        cancel_all(app.world_mut()).unwrap();
        assert!(!pending(app.world()));
        assert!(app.world().get_entity(document).is_err());
        assert_eq!(
            query(
                In(Some(json!({"session":"painting-test","stage":1}))),
                app.world_mut()
            )
            .unwrap()["status"],
            "canceled"
        );
    }

    #[test]
    fn completions_match_identity_clean_partial_files_and_ignore_duplicates() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        install(&mut app);
        let manager = JobManager::default();
        let (job, _) = manager
            .spawn_cpu::<(), _, _, String>(Default::default(), |_| async { Ok(()) })
            .unwrap();
        let directory = tempfile::tempdir().unwrap();
        let document = app.world_mut().spawn(CanvasDocument).id();
        let request = app.world_mut().spawn_empty().id();
        let path = directory.path().join("stage_0001_1x.png");
        let temp = temporary(&path).unwrap();
        std::fs::write(&temp, b"partial").unwrap();
        app.world_mut().resource_mut::<Stages>().0.insert(
            ("painting-test".into(), 1),
            Stage {
                document,
                region: Rect::from_corners(Vec2::ZERO, Vec2::splat(10.0)),
                files: vec![StageFile {
                    path: path.clone(),
                    scale: Some(1),
                    size: UVec2::splat(10),
                    state: FileState::Exporting {
                        request,
                        job,
                        temporary: temp.clone(),
                    },
                }],
            },
        );
        let wrong_document = app.world_mut().spawn(CanvasDocument).id();
        app.world_mut().trigger(CanvasPngExportFinished {
            entity: request,
            job_id: job,
            document: wrong_document,
            path: temp.clone(),
            result: Ok(UVec2::splat(10)),
        });
        assert!(app.world().resource::<Stages>().0[&("painting-test".into(), 1)].pending());
        app.world_mut().trigger(CanvasPngExportFinished {
            entity: request,
            job_id: job,
            document,
            path: temp.clone(),
            result: Err(CanvasExportError::Canceled),
        });
        app.world_mut().flush();
        assert!(!temp.exists());
        assert!(!path.exists());
        assert!(!pending(app.world()));
        let response = query(
            In(Some(json!({"session":"painting-test","stage":1}))),
            app.world_mut(),
        )
        .unwrap();
        assert_eq!(response["status"], "canceled");
        app.world_mut().trigger(CanvasPngExportFinished {
            entity: request,
            job_id: job,
            document,
            path: temp,
            result: Ok(UVec2::splat(10)),
        });
        assert_eq!(
            query(
                In(Some(json!({"session":"painting-test","stage":1}))),
                app.world_mut()
            )
            .unwrap(),
            response
        );
    }

    #[test]
    fn stage_keeps_fixed_svg_region_and_reports_png_submission_failures() {
        let mut app = App::new();
        app.register_type::<CanvasGeometry>()
            .register_type::<CanvasPaint>()
            .register_type::<Transform>();
        app.init_resource::<Sessions>()
            .add_message::<RequestRedraw>();
        install(&mut app);
        let directory = tempfile::tempdir().unwrap();
        let document = app.world_mut().spawn(CanvasDocument).id();
        let shape = CanvasDocument::create_rectangle(
            app.world_mut(),
            document,
            CanvasRectangle {
                size: Vec2::splat(10.0),
                corner_radius: 0.0,
            },
            Transform::default(),
            CanvasPaint::default(),
        )
        .unwrap();
        app.world_mut().resource_mut::<Sessions>().0.insert(
            "painting-test".into(),
            Session {
                document,
                size: UVec2::new(100, 80),
                directory: directory.path().into(),
                window: None,
                shapes: BTreeMap::from([("shape".into(), shape)]),
                next_stage: 1,
                dirty: true,
                save_error: None,
            },
        );
        let first = export(
            In(Some(json!({"session":"painting-test"}))),
            app.world_mut(),
        )
        .unwrap();
        assert_eq!(first["stage"], 1);
        assert_eq!(first["status"], "partial_failure");
        assert_eq!(first["files"][0]["status"], "succeeded");
        for file in first["files"].as_array().unwrap().iter().skip(1) {
            assert_eq!(file["status"], "failed");
            assert!(
                file["error"]
                    .as_str()
                    .unwrap()
                    .contains("renderer unavailable")
            );
            assert!(!Path::new(file["path"].as_str().unwrap()).exists());
        }
        let svg = std::fs::read_to_string(directory.path().join("stages/stage_0001.svg")).unwrap();
        assert!(svg.contains("viewBox=\"0 0 100 80\""));
        assert!(svg.contains("width=\"100\" height=\"80\""));
        let second = export(
            In(Some(json!({"session":"painting-test"}))),
            app.world_mut(),
        )
        .unwrap();
        assert_eq!(second["stage"], 2);
        assert_eq!(
            std::fs::read_to_string(directory.path().join("stages/stage_0001.svg")).unwrap(),
            svg
        );
        let stored: Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("document.json")).unwrap())
                .unwrap();
        assert_eq!(stored["next_stage"], 3);
    }
}
