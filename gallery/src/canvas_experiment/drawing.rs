const MAX_BATCH_OPERATIONS: usize = 1024;
const MAX_SHAPE_ID_BYTES: usize = 128;

use super::*;
use bevy::reflect::serde::{TypedReflectDeserializer, TypedReflectSerializer};
use bevy_widgetry::canvas::{CanvasGeometry, CanvasOrder, CanvasPaint};
use serde::Serialize;
use serde::de::DeserializeSeed;
use std::any::TypeId;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    session: String,
    operations: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Create {
        id: String,
        geometry: Value,
        paint: Option<Value>,
        transform: Option<Value>,
    },
    Update {
        id: String,
        geometry: Option<Value>,
        paint: Option<Value>,
        transform: Option<Value>,
    },
    Delete {
        id: String,
    },
    Order {
        id: String,
        order: Order,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Order {
    Raise,
    Lower,
    Front,
    Back,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    session: String,
    ids: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ShapeRecord {
    pub(super) id: String,
    pub(super) geometry: Value,
    pub(super) paint: Value,
    pub(super) transform: Value,
}

pub(super) fn batch(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Batch = params(value).inspect_err(|error| {
        info!(
            operation = "batch",
            origin = "brp",
            result = "rejected",
            code = error.code,
            "绘图批次参数被拒绝"
        );
    })?;
    if request.operations.len() > MAX_BATCH_OPERATIONS {
        info!(
            session = request.session,
            operation = "batch",
            origin = "brp",
            total = request.operations.len(),
            result = "rejected",
            reason = "too_many_operations",
            "绘图批次请求被拒绝"
        );
        return Err(invalid_params("A batch accepts at most 1024 operations"));
    }
    world.resource_scope(|world, mut sessions: Mut<Sessions>| {
        let session = sessions.0.get_mut(&request.session).ok_or_else(|| {
            info!(session=request.session, operation="batch", origin="brp", total=request.operations.len(),
                result="rejected", reason="unknown_session", "绘图批次请求被拒绝");
            invalid_params("Unknown session")
        })?;
        info!(session=request.session, operation="batch", origin="brp", total=request.operations.len(),
            result="accepted", "绘图批次已接受");
        let mut results = Vec::with_capacity(request.operations.len());
        let mut successful = 0;
        for (index, operation) in request.operations.into_iter().enumerate() {
            let result = serde_json::from_value::<Operation>(operation).map_err(invalid_params)
                .and_then(|operation| apply(world, session, operation));
            results.push(match result {
                Ok(id) => { successful += 1; json!({"index": index, "ok": true, "id": id}) }
                Err(error) => {
                    json!({"index": index, "ok": false, "error": error})
                }
            });
        }
        world.write_message(RequestRedraw);
        super::storage::persist(world, &request.session, session);
        let failed = results.len() - successful;
        let all_succeeded = failed == 0;
        let outcome = if all_succeeded { "succeeded" } else if successful == 0 { "failed" } else { "partial_failure" };
        info!(session = request.session, operation="batch", origin="brp", successful, failed, total = results.len(),
            all_succeeded, saved=!session.dirty, error=?session.save_error, result=outcome, "绘图批次执行结果");
        Ok(json!({"session": request.session, "results": results, "successful": successful,
            "all_succeeded": all_succeeded,
            "shape_count": CanvasDocument::shape_order(world, session.document).map_err(crate::diagnostics::managed_response_error)?.len(),
            "persistence": {"saved": !session.dirty, "error": session.save_error}}))
    })
}

pub(super) fn query(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Query = params(value)?;
    world.resource_scope(|world, sessions: Mut<Sessions>| {
        let session = sessions
            .0
            .get(&request.session)
            .ok_or_else(|| invalid_params("Unknown session"))?;
        if let Some(ids) = request.ids {
            let shapes = ids
                .iter()
                .map(|id| record(world, session, id))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(json!({"session": request.session, "shapes": shapes}))
        } else {
            let reverse: BTreeMap<_, _> = session
                .shapes
                .iter()
                .map(|(id, entity)| (*entity, id))
                .collect();
            let order = CanvasDocument::shape_order(world, session.document)
                .map_err(crate::diagnostics::managed_response_error)?;
            let ids: Vec<_> = order
                .into_iter()
                .filter_map(|entity| reverse.get(&entity).copied())
                .collect();
            Ok(json!({"session": request.session, "ids": ids}))
        }
    })
}

fn apply(
    world: &mut World,
    session: &mut Session,
    operation: Operation,
) -> Result<String, BrpError> {
    match operation {
        Operation::Create {
            id,
            geometry,
            paint,
            transform,
        } => {
            if !valid_id(&id) {
                return Err(invalid_params(
                    "Shape ID must contain 1..=128 ASCII letters, digits, underscores or hyphens",
                ));
            }
            if session.shapes.contains_key(&id) {
                return Err(invalid_params(
                    "Shape ID has already been used in this session",
                ));
            }
            let geometry = decode::<CanvasGeometry>(world, geometry)?;
            let paint = paint
                .map(|value| decode::<CanvasPaint>(world, value))
                .transpose()?
                .unwrap_or_default();
            let transform = transform
                .map(|value| decode::<Transform>(world, value))
                .transpose()?
                .unwrap_or_default();
            let entity = create_shape(world, session.document, geometry, paint, transform)
                .map_err(crate::diagnostics::managed_response_error)?;
            session.shapes.insert(id.clone(), entity);
            Ok(id)
        }
        Operation::Update {
            id,
            geometry,
            paint,
            transform,
        } => {
            let entity = entity(world, session, &id)?;
            let geometry = match geometry {
                Some(value) => decode(world, value)?,
                None => component::<CanvasGeometry>(world, entity)?.clone(),
            };
            let paint = match paint {
                Some(value) => decode(world, value)?,
                None => *component::<CanvasPaint>(world, entity)?,
            };
            let transform = match transform {
                Some(value) => decode(world, value)?,
                None => *component::<Transform>(world, entity)?,
            };
            let result = match geometry {
                CanvasGeometry::Rectangle(rectangle) => CanvasDocument::update_rectangle(
                    world,
                    session.document,
                    entity,
                    rectangle,
                    transform,
                    paint,
                ),
                CanvasGeometry::Ellipse(ellipse) => CanvasDocument::update_ellipse(
                    world,
                    session.document,
                    entity,
                    ellipse,
                    transform,
                    paint,
                ),
                CanvasGeometry::Path(path) => CanvasDocument::update_path(
                    world,
                    session.document,
                    entity,
                    path,
                    transform,
                    paint,
                ),
            };
            result.map_err(crate::diagnostics::managed_response_error)?;
            Ok(id)
        }
        Operation::Delete { id } => {
            let entity = entity(world, session, &id)?;
            CanvasDocument::remove_shape(world, session.document, entity)
                .map_err(crate::diagnostics::managed_response_error)?;
            Ok(id)
        }
        Operation::Order { id, order } => {
            let entity = entity(world, session, &id)?;
            let order = match order {
                Order::Raise => CanvasOrder::Raise,
                Order::Lower => CanvasOrder::Lower,
                Order::Front => CanvasOrder::Front,
                Order::Back => CanvasOrder::Back,
            };
            CanvasDocument::reorder_shape(world, session.document, entity, order)
                .map_err(crate::diagnostics::managed_response_error)?;
            Ok(id)
        }
    }
}

pub(super) fn create_shape(
    world: &mut World,
    document: Entity,
    geometry: CanvasGeometry,
    paint: CanvasPaint,
    transform: Transform,
) -> Result<Entity, bevy_widgetry::job::JobFailure> {
    match geometry {
        CanvasGeometry::Rectangle(rectangle) => {
            CanvasDocument::create_rectangle(world, document, rectangle, transform, paint)
        }
        CanvasGeometry::Ellipse(ellipse) => {
            CanvasDocument::create_ellipse(world, document, ellipse, transform, paint)
        }
        CanvasGeometry::Path(path) => {
            CanvasDocument::create_path(world, document, path, transform, paint)
        }
    }
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_SHAPE_ID_BYTES
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}

fn entity(world: &World, session: &Session, id: &str) -> Result<Entity, BrpError> {
    session
        .shapes
        .get(id)
        .copied()
        .filter(|entity| {
            world.get::<CanvasGeometry>(*entity).is_some()
                && world
                    .get::<ChildOf>(*entity)
                    .is_some_and(|parent| parent.parent() == session.document)
        })
        .ok_or_else(|| invalid_params(format!("Unknown or deleted shape: {id}")))
}

fn component<T: Component>(world: &World, entity: Entity) -> Result<&T, BrpError> {
    world.get::<T>(entity).ok_or_else(|| {
        crate::diagnostics::response_error(format!(
            "Shape {} missing {}",
            entity,
            std::any::type_name::<T>()
        ))
    })
}

pub(super) fn record(world: &World, session: &Session, id: &str) -> Result<ShapeRecord, BrpError> {
    let entity = entity(world, session, id)?;
    component::<CanvasGeometry>(world, entity)?
        .validate(
            component::<Transform>(world, entity)?,
            component::<CanvasPaint>(world, entity)?,
        )
        .map_err(crate::diagnostics::managed_response_error)?;
    Ok(ShapeRecord {
        id: id.into(),
        geometry: encode(world, component::<CanvasGeometry>(world, entity)?)?,
        paint: encode(world, component::<CanvasPaint>(world, entity)?)?,
        transform: encode(world, component::<Transform>(world, entity)?)?,
    })
}

fn encode<T: Reflect>(world: &World, value: &T) -> Result<Value, BrpError> {
    let registry = world.resource::<AppTypeRegistry>().read();
    serde_json::to_value(TypedReflectSerializer::new(value, &registry))
        .map_err(crate::diagnostics::response_error)
}

pub(super) fn decode<T: Reflect + FromReflect>(world: &World, value: Value) -> Result<T, BrpError> {
    let registry = world.resource::<AppTypeRegistry>().read();
    let registration = registry
        .get(TypeId::of::<T>())
        .ok_or_else(|| crate::diagnostics::response_error("Drawing type is not registered"))?;
    let reflected = TypedReflectDeserializer::new(registration, &registry)
        .deserialize(value)
        .map_err(invalid_params)?;
    T::from_reflect(reflected.as_ref())
        .ok_or_else(|| invalid_params("Drawing value cannot be reconstructed"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry::canvas::*;

    #[derive(Resource)]
    struct TestDirectory {
        _directory: tempfile::TempDir,
    }

    fn app() -> App {
        let mut app = App::new();
        app.register_type::<CanvasGeometry>()
            .register_type::<CanvasPaint>()
            .register_type::<Transform>();
        app.init_resource::<Sessions>()
            .add_message::<RequestRedraw>();
        let document = app.world_mut().spawn(CanvasDocument).id();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().to_owned();
        app.insert_resource(TestDirectory {
            _directory: directory,
        });
        app.world_mut().resource_mut::<Sessions>().0.insert(
            "test".into(),
            Session {
                document,
                size: UVec2::new(100, 100),
                directory: path,
                window: None,
                shapes: BTreeMap::new(),
                next_stage: 1,
                dirty: true,
                save_error: None,
            },
        );
        app
    }

    #[test]
    fn batch_logs_match_responses_without_warning_for_expected_rejections() {
        use bevy_widgetry_test_utils::LogCapture;
        let mut app = app();
        let logs = LogCapture::with_targets([
            "widget_gallery::canvas_experiment::drawing",
            "widget_gallery",
            "bevy_widgetry",
        ]);
        let responses = logs.run(|| {
            let mut responses = Vec::new();
            for operations in [
                json!([
                    {"op":"create","id":"first","geometry":{"Rectangle":{"size":[10,10],"corner_radius":0}}},
                    {"op":"delete","id":"missing"},
                    {"op":"nonsense"}
                ]),
                json!([]),
                json!([{"op":"delete","id":"missing"}]),
                json!([{"op":"delete","id":"first"}]),
            ] {
                responses.push(batch(In(Some(json!({"session":"test","operations":operations}))), app.world_mut()).unwrap());
            }
            for request in [
                json!({"session":"test","operations":vec![Value::Null; 1025]}),
                json!({"session":"missing","operations":[]}),
                json!({"session":"test","operations":"invalid"}),
            ] {
                assert!(batch(In(Some(request)), app.world_mut()).is_err());
            }
            responses
        });
        let records = logs.records();
        assert!(
            !records
                .iter()
                .any(|r| r.level == bevy::log::tracing::Level::WARN
                    || r.level == bevy::log::tracing::Level::ERROR)
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.matches_field("result", "accepted"))
                .count(),
            4
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.matches_field("result", "rejected"))
                .count(),
            3
        );
        let summaries: Vec<_> = records
            .iter()
            .filter(|r| r.fields.contains_key("all_succeeded"))
            .collect();
        assert_eq!(summaries.len(), responses.len());
        for (summary, response) in summaries.into_iter().zip(responses) {
            let total = response["results"].as_array().unwrap().len();
            let successful = response["successful"].as_u64().unwrap() as usize;
            assert!(summary.matches_field("session", "test"));
            assert!(summary.matches_field("origin", "brp"));
            for (field, value) in [
                ("total", total),
                ("successful", successful),
                ("failed", total - successful),
            ] {
                assert!(summary.matches_field(field, &value.to_string()));
            }
            assert!(summary.matches_field("all_succeeded", &response["all_succeeded"].to_string()));
            assert!(summary.matches_field("saved", &response["persistence"]["saved"].to_string()));
        }
    }

    #[test]
    fn batch_keeps_real_failure_diagnostics_and_continues_after_mixed_errors() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
        let mut app = app();
        let geometry = json!({"Rectangle":{"size":[10,10],"corner_radius":0}});
        batch(In(Some(json!({"session":"test","operations":[{"op":"create","id":"broken","geometry":geometry}]}))), app.world_mut()).unwrap();
        let broken = app.world().resource::<Sessions>().0["test"].shapes["broken"];
        app.world_mut().entity_mut(broken).remove::<CanvasPaint>();
        let logs = LogCapture::with_targets([
            "widget_gallery::canvas_experiment::drawing",
            "widget_gallery",
            "bevy_widgetry",
        ]);
        let response = logs.run(|| {
            batch(
                In(Some(json!({"session":"test","operations":[
                    {"op":"nonsense"}, {"op":"update","id":"broken"},
                    {"op":"create","id":"good","geometry":geometry}
                ]}))),
                app.world_mut(),
            )
            .unwrap()
        });
        assert_eq!(response["successful"], 1);
        assert_eq!(response["all_succeeded"], false);
        assert_eq!(response["shape_count"], 1);
        assert_eq!(response["persistence"]["saved"], true);
        let records = logs.records();
        assert!(
            !records
                .iter()
                .any(|r| r.level == bevy::log::tracing::Level::WARN)
        );
        let sources: Vec<_> = records
            .iter()
            .filter(|r| r.phase() == Some(DiagnosticPhase::Source))
            .collect();
        assert_eq!(sources.len(), 1);
        for source in sources {
            assert_eq!(source.level, bevy::log::tracing::Level::ERROR);
            assert_eq!(
                records
                    .iter()
                    .filter(|r| r.phase() == Some(DiagnosticPhase::Handled)
                        && r.fields.get("error_id") == source.fields.get("error_id"))
                    .count(),
                1
            );
        }
        let summary = records
            .iter()
            .find(|r| r.fields.contains_key("all_succeeded"))
            .unwrap();
        assert!(summary.matches_field("successful", "1"));
        assert!(summary.matches_field("failed", "2"));
    }

    #[test]
    fn successful_batch_with_failed_persistence_keeps_edits_and_logs_saved_false() {
        use bevy_widgetry_test_utils::LogCapture;
        use std::os::windows::fs::OpenOptionsExt;
        let mut app = app();
        batch(
            In(Some(json!({"session":"test","operations":[]}))),
            app.world_mut(),
        )
        .unwrap();
        let path = app.world().resource::<Sessions>().0["test"]
            .directory
            .join("document.json");
        let previous = std::fs::read(&path).unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        let logs = LogCapture::with_targets([
            "widget_gallery::canvas_experiment::drawing",
            "widget_gallery",
            "bevy_widgetry",
        ]);
        let response = logs.run(|| batch(In(Some(json!({"session":"test","operations":[
            {"op":"create","id":"unsaved","geometry":{"Rectangle":{"size":[10,10],"corner_radius":0}}}
        ]}))), app.world_mut()).unwrap());
        assert_eq!(response["successful"], 1);
        assert_eq!(response["all_succeeded"], true);
        assert_eq!(response["persistence"]["saved"], false);
        assert_eq!(response["shape_count"], 1);
        drop(locked);
        assert_eq!(std::fs::read(path).unwrap(), previous);
        let records = logs.records();
        let summary = records
            .iter()
            .find(|r| r.fields.contains_key("all_succeeded"))
            .unwrap();
        assert!(summary.matches_field("all_succeeded", "true"));
        assert!(summary.matches_field("saved", "false"));
        assert!(!summary.matches_field("error", "None"));
        assert_eq!(
            records
                .iter()
                .filter(|r| r.level == bevy::log::tracing::Level::ERROR)
                .count(),
            1
        );
    }

    #[test]
    fn batch_keeps_successful_operations_and_continues_after_errors() {
        let mut app = app();
        let rectangle = encode(
            app.world(),
            &CanvasGeometry::Rectangle(CanvasRectangle {
                size: Vec2::splat(20.0),
                corner_radius: 2.0,
            }),
        )
        .unwrap();
        let result = batch(
            In(Some(json!({"session":"test","operations":[
                {"op":"create","id":"first","geometry":rectangle},
                {"op":"create","id":"first","geometry":rectangle},
                {"op":"nonsense"},
                {"op":"create","id":"last","geometry":rectangle},
                {"op":"order","id":"last","order":"back"}
            ]}))),
            app.world_mut(),
        )
        .unwrap();
        assert_eq!(result["successful"], 3);
        assert_eq!(result["all_succeeded"], false);
        assert_eq!(result["persistence"]["saved"], true);
        let path = app.world().resource::<Sessions>().0["test"]
            .directory
            .join("document.json");
        let archive: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(archive["shapes"].as_array().unwrap().len(), 2);
        assert_eq!(archive["shapes"][0]["id"], "last");
        let queried = query(In(Some(json!({"session":"test"}))), app.world_mut()).unwrap();
        assert_eq!(queried["ids"], json!(["last", "first"]));
        let result = batch(In(Some(json!({"session":"test","operations":[{"op":"delete","id":"first"},{"op":"update","id":"first"}]}))), app.world_mut()).unwrap();
        assert_eq!(result["successful"], 1);
    }

    #[test]
    fn drawing_json_preserves_curves_paint_and_transform() {
        let app = app();
        let path = CanvasGeometry::Path(CanvasPath {
            commands: vec![
                CanvasPathCommand::MoveTo(Vec2::ZERO),
                CanvasPathCommand::QuadTo {
                    control: Vec2::new(2.0, 9.0),
                    to: Vec2::splat(10.0),
                },
                CanvasPathCommand::CubicTo {
                    control1: Vec2::X,
                    control2: Vec2::Y,
                    to: Vec2::splat(4.0),
                },
                CanvasPathCommand::ArcTo {
                    radii: Vec2::splat(3.0),
                    rotation: 0.3,
                    large_arc: true,
                    sweep: false,
                    to: Vec2::ZERO,
                },
                CanvasPathCommand::Close,
                CanvasPathCommand::MoveTo(Vec2::splat(1.0)),
                CanvasPathCommand::LineTo(Vec2::splat(2.0)),
            ],
        });
        let paint = CanvasPaint {
            fill: Some(Color::srgba(0.2, 0.3, 0.4, 0.5)),
            fill_rule: CanvasFillRule::EvenOdd,
            stroke: Some(CanvasStroke {
                color: Color::linear_rgba(0.1, 0.2, 0.3, 0.4),
                width: 2.0,
                cap: CanvasLineCap::Round,
                join: CanvasLineJoin::Bevel,
            }),
        };
        let transform = Transform::from_xyz(1.0, 2.0, 3.0)
            .with_rotation(Quat::from_rotation_z(0.6))
            .with_scale(Vec3::new(-2.0, 0.5, 1.0));
        assert_eq!(
            decode::<CanvasGeometry>(app.world(), encode(app.world(), &path).unwrap()).unwrap(),
            path
        );
        assert_eq!(
            decode::<CanvasPaint>(app.world(), encode(app.world(), &paint).unwrap()).unwrap(),
            paint
        );
        assert_eq!(
            decode::<Transform>(app.world(), encode(app.world(), &transform).unwrap()).unwrap(),
            transform
        );
    }

    #[test]
    #[ignore = "Manual optimized CPU benchmark; excludes rendering and transport"]
    fn benchmark_batch_throughput() {
        use std::fmt::Write;
        use std::time::Instant;
        let artifact = bevy_widgetry_test_utils::benchmark::artifact::Artifact::new(
            "canvas-experiment-batch", "optimized dev test profile; request decoding, public CanvasDocument operations and JSON persistence; no App frames/GPU/BRP transport"
        ).unwrap();
        let mut csv = String::from("shapes,sample,create_us,update_us,delete_us\n");
        for count in [100, 1000] {
            for sample in 0..120 {
                let mut app = app();
                let rectangle = encode(
                    app.world(),
                    &CanvasGeometry::Rectangle(CanvasRectangle {
                        size: Vec2::splat(20.0),
                        corner_radius: 2.0,
                    }),
                )
                .unwrap();
                let moved = encode(app.world(), &Transform::from_xyz(10.0, 20.0, 0.0)).unwrap();
                let create: Vec<_> = (0..count)
                    .map(|id| json!({"op":"create","id":format!("shape{id}"),"geometry":rectangle}))
                    .collect();
                let update: Vec<_> = (0..count)
                    .map(|id| json!({"op":"update","id":format!("shape{id}"),"transform":moved}))
                    .collect();
                let delete: Vec<_> = (0..count)
                    .map(|id| json!({"op":"delete","id":format!("shape{id}")}))
                    .collect();
                let mut times = Vec::new();
                for operations in [create, update, delete] {
                    let request = json!({"session":"test","operations":operations});
                    let start = Instant::now();
                    let response = batch(In(Some(request)), app.world_mut()).unwrap();
                    times.push(start.elapsed().as_micros());
                    assert_eq!(response["successful"], count);
                    assert_eq!(response["all_succeeded"], true);
                }
                if sample >= 20 {
                    writeln!(
                        csv,
                        "{count},{},{},{},{}",
                        sample - 20,
                        times[0],
                        times[1],
                        times[2]
                    )
                    .unwrap();
                }
            }
        }
        std::fs::write(artifact.directory.join("samples.csv"), csv).unwrap();
        println!(
            "Canvas batch CPU benchmark: {}",
            artifact.directory.display()
        );
    }
}
