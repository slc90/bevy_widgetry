use bevy::prelude::*;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};
use bevy_widgetry_test_utils::scene_app;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;

struct Fixture {
    app: App,
    root: Entity,
    data: Vec<WidgetryFileDialogEntryData>,
    snapshot: Option<Arc<WidgetryFileDialogSnapshot>>,
}

fn fixture(items: usize, loaded: bool) -> Result<Fixture> {
    let mut app = scene_app();
    app.add_plugins(WidgetryFileDialogHeadlessPlugin);
    let path = PathBuf::from(if cfg!(windows) {
        "C:/fixture"
    } else {
        "/fixture"
    });
    let root = app
        .world_mut()
        .spawn_scene(
            bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::PickFiles,
            @initial_directory: {Some(path.clone())} } },
        )?
        .id();
    let data = (0..items)
        .map(|index| {
            let name = format!("file_{index:06}.txt");
            WidgetryFileDialogEntryData {
                path: path.join(&name),
                name: name.into(),
                kind: WidgetryFileDialogEntryKind::File,
                size: Some(index as u64),
                modified: None,
                hidden: Some(false),
                system: Some(false),
            }
        })
        .collect();
    let mut fixture = Fixture {
        app,
        root,
        data,
        snapshot: None,
    };
    if loaded {
        initial(&mut fixture)?;
    }
    Ok(fixture)
}

fn initial(fixture: &mut Fixture) -> Result {
    let state = fixture
        .app
        .world()
        .get::<WidgetryFileDialogState>(fixture.root)
        .ok_or_else(|| missing("FileDialogState"))?;
    let token = state.token();
    let path = state
        .requested_path()
        .ok_or_else(|| missing("requested path"))?
        .to_owned();
    let snapshot = Arc::new(WidgetryFileDialogSnapshot::prepare(
        token,
        path.clone(),
        std::mem::take(&mut fixture.data),
        &state.query(),
        None,
    )?);
    WidgetryFileDialog::deliver(
        fixture.app.world_mut(),
        fixture.root,
        WidgetryFileDialogReply::Started { token, path },
    )?;
    install(fixture, snapshot)
}

fn install(fixture: &mut Fixture, snapshot: Arc<WidgetryFileDialogSnapshot>) -> Result {
    let selection = fixture
        .app
        .world()
        .get::<WidgetryFileDialogState>(fixture.root)
        .ok_or_else(|| missing("FileDialogState"))?
        .projection_selection_job(snapshot.clone())?
        .prepare()?;
    WidgetryFileDialog::deliver(
        fixture.app.world_mut(),
        fixture.root,
        WidgetryFileDialogReply::Snapshot {
            snapshot: snapshot.clone(),
            state: WidgetryFileDialogDirectoryState::Ready,
            selection,
        },
    )?;
    fixture.snapshot = Some(snapshot);
    Ok(())
}

fn main() -> Result {
    let mut harness = Harness::new("file_dialog-headless-criterion")?;
    for items in [1_000, 10_000, 100_000] {
        for action in ["initial_snapshot", "cached_projection", "bulk_selection"] {
            run(
                &mut harness,
                &format!("file_dialog/n{items}/{action}"),
                action == "initial_snapshot",
                || fixture(items, action != "initial_snapshot"),
                |fixture, iteration| {
                    match action {
                        "initial_snapshot" => initial(fixture)?,
                        "cached_projection" => {
                            WidgetryFileDialog::apply(
                                fixture.app.world_mut(),
                                fixture.root,
                                WidgetryFileDialogAction::Search(if iteration.is_multiple_of(2) {
                                    "000".into()
                                } else {
                                    String::new()
                                }),
                            )?;
                            let state = fixture
                                .app
                                .world()
                                .get::<WidgetryFileDialogState>(fixture.root)
                                .ok_or_else(|| missing("FileDialogState"))?;
                            let snapshot = Arc::new(
                                fixture
                                    .snapshot
                                    .as_ref()
                                    .ok_or_else(|| missing("snapshot"))?
                                    .reproject(state.token(), &state.query())?,
                            );
                            install(fixture, snapshot)?;
                        }
                        "bulk_selection" => {
                            WidgetryFileDialog::apply(
                                fixture.app.world_mut(),
                                fixture.root,
                                if iteration.is_multiple_of(2) {
                                    WidgetryFileDialogAction::SelectAll
                                } else {
                                    WidgetryFileDialogAction::ClearSelection
                                },
                            )?;
                            let job = fixture
                                .app
                                .world()
                                .get::<WidgetryFileDialogState>(fixture.root)
                                .ok_or_else(|| missing("FileDialogState"))?
                                .selection_job()
                                .ok_or_else(|| missing("selection job"))?;
                            WidgetryFileDialog::deliver(
                                fixture.app.world_mut(),
                                fixture.root,
                                WidgetryFileDialogReply::Selection(job.prepare()?),
                            )?;
                        }
                        _ => return Err(missing("benchmark action")),
                    }
                    let state = fixture
                        .app
                        .world()
                        .get::<WidgetryFileDialogState>(fixture.root)
                        .ok_or_else(|| missing("FileDialogState"))?;
                    black_box((state.visible().len(), state.selected().len()));
                    Ok(())
                },
                |fixture| Ok(fixture.app.world().entities().count_spawned()),
            )?;
        }
    }
    harness.finish()
}
