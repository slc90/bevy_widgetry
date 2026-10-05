use bevy::diagnostic::FrameCount;
use bevy::input::{ButtonState, keyboard::KeyboardInput, mouse::MouseButtonInput};
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems, camera::ExtractedCamera,
    sync_world::MainEntity, view::ViewTarget,
};
use bevy::ui_render::{ExtractedUiItem, ExtractedUiNodes};
use bevy_widgetry::file_dialog::*;
use bevy_widgetry::window::widgetry_window_target;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

#[derive(Resource, Clone)]
struct Sink {
    epoch: Instant,
    output: mpsc::SyncSender<Value>,
    failure: Arc<Mutex<Option<String>>>,
}
impl Sink {
    fn ns(&self, time: Instant) -> u64 {
        time.saturating_duration_since(self.epoch)
            .as_nanos()
            .min(u128::from(u64::MAX)) as u64
    }
    fn send(&self, value: Value) -> Result {
        self.output.try_send(value).map_err(|error| {
            BevyError::error(format!("FileDialog benchmark output queue failed: {error}"))
        })
    }
}

#[derive(Clone)]
struct Sample {
    id: u64,
    root: Entity,
    session: String,
    camera: Option<Entity>,
    native: Option<Entity>,
    recorded: BTreeSet<&'static str>,
}
#[derive(Resource, Default)]
struct Measurement {
    serial: u64,
    input: BTreeMap<Entity, (u32, Instant)>,
    samples: Vec<Sample>,
    fixture: Option<PathBuf>,
}
#[derive(Resource, Default, Clone)]
struct Probes(Vec<Probe>, BTreeSet<u64>);
#[derive(Clone)]
struct Probe {
    sample: u64,
    root: Entity,
    session: String,
    camera: Entity,
    native: Entity,
    frame: u32,
    glyphs: Vec<Entity>,
}
#[derive(Resource, Default)]
struct RenderEvidence {
    candidates: Vec<Probe>,
    completed: BTreeSet<u64>,
}

pub(crate) fn install(app: &mut App) -> Result {
    let Some(output) = std::env::var_os("GALLERY_FILE_DIALOG_BENCH_OUTPUT") else {
        return Ok(());
    };
    let output = PathBuf::from(output);
    std::fs::create_dir_all(&output)?;
    let file = std::fs::File::create_new(output.join("events.jsonl"))?;
    let (sender, receiver) = mpsc::sync_channel::<Value>(2048);
    let failure = Arc::new(Mutex::new(None));
    let writer_failure = failure.clone();
    std::thread::Builder::new()
        .name("gallery-file-dialog-trace".into())
        .spawn(move || {
            let result = (|| -> std::io::Result<()> {
                let mut file = BufWriter::new(file);
                for value in receiver {
                    serde_json::to_writer(&mut file, &value)?;
                    writeln!(file)?;
                    file.flush()?;
                }
                Ok(())
            })();
            if let Err(error) = result {
                error!(%error,"FileDialog benchmark writer failed");
                if let Ok(mut failure) = writer_failure.lock() {
                    *failure = Some(error.to_string());
                }
            }
        })?;
    let sink = Sink {
        epoch: Instant::now(),
        output: sender,
        failure,
    };
    let clock_id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    sink.send(json!({"protocol":1,"event":"protocol","clock":"app_monotonic_ns","clock_id":clock_id,"t_presented":null,"first_content_frame_endpoint":"post_render_graph_with_extracted_content","display_evidence":"required_external_presentation_adapter"}))?;
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return Err(BevyError::error(
            "FileDialog GUI benchmark requires RenderApp",
        ));
    };
    render_app
        .insert_resource(sink.clone())
        .init_resource::<Probes>()
        .init_resource::<RenderEvidence>()
        .add_systems(ExtractSchedule, extract_probes)
        .add_systems(
            Render,
            identify_content
                .after(RenderSystems::Queue)
                .before(RenderSystems::Prepare),
        )
        .add_systems(
            Render,
            submitted
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        );
    app.insert_resource(sink)
        .insert_resource(Measurement {
            fixture: std::env::var_os("GALLERY_FILE_DIALOG_BENCH_FIXTURE").map(PathBuf::from),
            ..default()
        })
        .init_resource::<Probes>()
        .add_systems(
            First,
            input_boundary.before(bevy::picking::PickingSystems::Input),
        )
        .add_systems(
            PostUpdate,
            observe
                .after(bevy::ui::UiSystems::PostLayout)
                .after(bevy::camera::CameraUpdateSystems),
        )
        .add_systems(Last, writer_status);
    Ok(())
}

fn writer_status(sink: Res<Sink>) -> Result {
    let mut failure = sink.failure.lock().map_err(|error| {
        BevyError::error(format!(
            "FileDialog benchmark writer status poisoned: {error}"
        ))
    })?;
    if let Some(error) = failure.take() {
        return Err(BevyError::error(format!(
            "FileDialog benchmark writer failed: {error}"
        )));
    }
    Ok(())
}

fn input_boundary(
    mut mouse: MessageReader<MouseButtonInput>,
    mut keys: MessageReader<KeyboardInput>,
    frame: Res<FrameCount>,
    mut measurement: ResMut<Measurement>,
) {
    measurement.input.clear();
    for input in mouse.read() {
        if input.button == MouseButton::Left && input.state == ButtonState::Released {
            measurement
                .input
                .insert(input.window, (frame.0, Instant::now()));
        }
    }
    for input in keys.read() {
        if input.state == ButtonState::Pressed
            && !input.repeat
            && matches!(input.key_code, KeyCode::Enter | KeyCode::Space)
        {
            measurement
                .input
                .insert(input.window, (frame.0, Instant::now()));
        }
    }
}

pub(crate) fn fixture_directory(world: &World) -> Option<PathBuf> {
    world
        .get_resource::<Measurement>()
        .and_then(|measurement| measurement.fixture.clone())
}

pub(crate) fn activate(
    world: &mut World,
    launcher: Entity,
    parent: Entity,
    operation: &str,
    time: Instant,
) -> Result<Option<u64>> {
    let Some(sink) = world.get_resource::<Sink>().cloned() else {
        return Ok(None);
    };
    let frame = world.resource::<FrameCount>().0;
    let mut measurement = world.resource_mut::<Measurement>();
    measurement.serial = measurement.serial.saturating_add(1);
    let id = measurement.serial;
    let input = measurement
        .input
        .get(&parent)
        .filter(|(input_frame, _)| *input_frame == frame)
        .map(|(_, time)| sink.ns(*time));
    // 无匹配raw input的程序化Activate不能伪装成有效latency样本。
    sink.send(json!({"event":"activate","sample":id,"operation":operation,"launcher":launcher.to_bits(),"input_window":parent.to_bits(),"app_frame":frame,"t_input":input,"t_activate":sink.ns(time),"t_presented":null}))?;
    Ok(Some(id))
}

pub(crate) fn scene_ready(
    world: &mut World,
    id: Option<u64>,
    root: Entity,
    session: WidgetryFileDialogSessionId,
) -> Result {
    let Some(id) = id else {
        return Ok(());
    };
    let Some(sink) = world.get_resource::<Sink>().cloned() else {
        return Ok(());
    };
    let session = format!("{session:?}");
    sink.send(json!({"event":"scene","sample":id,"root":root.to_bits(),"session":session,"t_scene":sink.ns(Instant::now())}))?;
    world.resource_mut::<Measurement>().samples.push(Sample {
        id,
        root,
        session,
        camera: None,
        native: None,
        recorded: BTreeSet::new(),
    });
    Ok(())
}

fn descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut pending = vec![root];
    let mut result = Vec::new();
    while let Some(entity) = pending.pop() {
        result.push(entity);
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
    }
    result
}

fn mark(sink: &Sink, sample: &mut Sample, name: &'static str, value: Value) -> Result {
    if sample.recorded.insert(name) {
        sink.send(json!({"event":name,"sample":sample.id,"root":sample.root.to_bits(),"session":sample.session,"native_window":sample.native.map(Entity::to_bits),"camera":sample.camera.map(Entity::to_bits),"ns":sink.ns(Instant::now()),"data":value}))?;
    }
    Ok(())
}

fn observe(world: &mut World) -> Result {
    let sink = world.resource::<Sink>().clone();
    let frame = world.resource::<FrameCount>().0;
    let mut samples = std::mem::take(&mut world.resource_mut::<Measurement>().samples);
    let mut probes = Vec::new();
    for sample in &mut samples {
        let Some(state) = world.get::<WidgetryFileDialogState>(sample.root) else {
            continue;
        };
        sample.native = widgetry_window_target(world, sample.root);
        sample.camera = world
            .get::<UiTargetCamera>(sample.root)
            .map(|target| target.0);
        if let (Some(native), Some(camera)) = (sample.native, sample.camera)
            && world.get::<Window>(native).is_some()
            && world
                .get::<Camera>(camera)
                .is_some_and(|camera| camera.is_active && camera.computed.target_info.is_some())
        {
            mark(&sink, sample, "camera_ready", json!({"app_frame":frame}))?;
        }
        if state.snapshot().is_some() {
            mark(
                &sink,
                sample,
                "first_batch",
                json!({"entry_count":state.entries().len()}),
            )?;
        }
        if !matches!(
            state.directory_state(),
            WidgetryFileDialogDirectoryState::Loading
        ) && !state.projection_pending()
        {
            mark(
                &sink,
                sample,
                "final_projection",
                json!({"state":format!("{:?}",state.directory_state()),"entry_count":state.entries().len(),"visible_count":state.visible().len()}),
            )?;
        }
        let (Some(native), Some(camera)) = (sample.native, sample.camera) else {
            continue;
        };
        let entities = descendants(world, sample.root);
        let named = |name: &str| {
            entities.iter().copied().find(|entity| {
                world
                    .get::<Name>(*entity)
                    .is_some_and(|value| value.as_str() == name)
            })
        };
        let (Some(path), Some(entries), Some(cancel), Some(status)) = (
            named("FileDialogPath"),
            named("FileDialogEntries"),
            named("FileDialogCancel"),
            named("FileDialogStatus"),
        ) else {
            continue;
        };
        if [path, entries, cancel, status].iter().any(|entity| {
            world
                .get::<ComputedNode>(*entity)
                .is_none_or(ComputedNode::is_empty)
                || world
                    .get::<InheritedVisibility>(*entity)
                    .is_some_and(|visibility| !visibility.get())
        }) || world.get::<bevy::ui::InteractionDisabled>(cancel).is_some()
        {
            continue;
        }
        let title = world
            .get::<Window>(native)
            .map(|window| window.title.as_str())
            .unwrap_or_default();
        let Some(title) = entities.iter().copied().find(|entity| {
            world
                .get::<Text>(*entity)
                .is_some_and(|text| text.0 == title)
        }) else {
            continue;
        };
        let Some(cancel_text) = descendants(world, cancel).into_iter().find(|entity| {
            world
                .get::<Text>(*entity)
                .is_some_and(|text| text.0 == "Cancel")
        }) else {
            continue;
        };
        let Some(path_label) = world
            .get::<ChildOf>(path)
            .and_then(|parent| world.get::<Children>(parent.parent()))
            .and_then(|children| {
                children.iter().find(|entity| {
                    world
                        .get::<Text>(*entity)
                        .is_some_and(|text| text.0 == "Path")
                })
            })
        else {
            continue;
        };
        let glyphs = vec![title, path_label, status, cancel_text];
        if glyphs.iter().any(|entity| {
            world
                .get::<bevy::text::TextLayoutInfo>(*entity)
                .is_none_or(|layout| layout.glyphs.is_empty())
        }) {
            continue;
        }
        mark(
            &sink,
            sample,
            "cpu_content_ready",
            json!({"app_frame":frame,"glyph_entities":glyphs.iter().map(|entity|entity.to_bits()).collect::<Vec<_>>()}),
        )?;
        probes.push(Probe {
            sample: sample.id,
            root: sample.root,
            session: sample.session.clone(),
            camera,
            native,
            frame,
            glyphs,
        });
    }
    samples.retain(|sample| world.get_entity(sample.root).is_ok());
    world.resource_mut::<Probes>().1 = samples.iter().map(|sample| sample.id).collect();
    world.resource_mut::<Measurement>().samples = samples;
    world.resource_mut::<Probes>().0 = probes;
    Ok(())
}

fn extract_probes(probes: Extract<Res<Probes>>, mut extracted: ResMut<Probes>) {
    *extracted = probes.clone();
}
fn identify_content(
    probes: Res<Probes>,
    nodes: Res<ExtractedUiNodes>,
    cameras: Query<(Entity, &MainEntity), With<ExtractedCamera>>,
    mut evidence: ResMut<RenderEvidence>,
) {
    evidence.candidates.clear();
    evidence
        .completed
        .retain(|sample| probes.1.contains(sample));
    for probe in &probes.0 {
        if evidence.completed.contains(&probe.sample) {
            continue;
        }
        let Some((camera, _)) = cameras.iter().find(|(_, main)| main.id() == probe.camera) else {
            continue;
        };
        if probe.glyphs.iter().all(|entity| {
            nodes.uinodes.iter().any(|node| {
                node.main_entity.id() == *entity
                    && node.extracted_camera_entity == camera
                    && matches!(&node.item,ExtractedUiItem::Glyphs {range} if !range.is_empty())
            })
        }) {
            evidence.candidates.push(probe.clone());
        }
    }
}
fn submitted(
    sink: Res<Sink>,
    views: Query<&MainEntity, With<ViewTarget>>,
    mut evidence: ResMut<RenderEvidence>,
) -> Result {
    let candidates = std::mem::take(&mut evidence.candidates);
    for probe in candidates {
        if !views.iter().any(|main| main.id() == probe.camera) {
            continue;
        }
        sink.send(json!({"event":"first_content_frame","sample":probe.sample,"root":probe.root.to_bits(),"session":probe.session,"native_window":probe.native.to_bits(),"camera":probe.camera.to_bits(),"app_frame":probe.frame,"ns":sink.ns(Instant::now()),"t_presented":null,"endpoint":"render_graph_submitted_with_content_extraction"}))?;
        evidence.completed.insert(probe.sample);
    }
    Ok(())
}
