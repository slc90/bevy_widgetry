use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

struct Fixture {
    app: App,
    root: Entity,
    reply: Option<WidgetryFileDialogReply>,
    // close 只计 UI hierarchy 的销毁。数据最终回收由 runtime benchmark 覆盖，避免 fixture 的最后一个 Arc 掩盖计时边界。
    retained: Vec<Arc<WidgetryFileDialogSnapshot>>,
    assets: Vec<Handle<bevy::asset::LoadedUntypedAsset>>,
}

fn fixture(items: usize, operation: &str) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..default()
    });
    app.add_plugins(WidgetryFileDialogPlugin);
    // 字体与 SVG 读取在 fixture 准备阶段完成，测量包含真实 row raster/layout，不包含 asset I/O 等待。
    let assets: Vec<_> = [
        BuiltinIcon::FileDialogFile,
        BuiltinIcon::FileDialogFolder,
        BuiltinIcon::FileDialogBack,
        BuiltinIcon::FileDialogForward,
        BuiltinIcon::FileDialogUp,
        BuiltinIcon::FileDialogRefresh,
    ]
    .into_iter()
    .map(|icon| {
        app.world()
            .resource::<AssetServer>()
            .load_builder()
            .load_untyped(icon.path())
    })
    .collect();
    bevy_widgetry_test_utils::advance_until(
        &mut app,
        Duration::from_secs(10),
        "FileDialog SVG",
        |world| {
            assets.iter().all(|asset| {
                world
                    .resource::<Assets<bevy::asset::LoadedUntypedAsset>>()
                    .contains(asset)
            })
        },
    )
    .map_err(BevyError::error)?;
    let root = app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @mode: WidgetryFileDialogMode::PickFiles } Node {width: px(900), height: px(700)} })?.id();
    let mut fixture = Fixture {
        app,
        root,
        reply: None,
        retained: Vec::new(),
        assets,
    };
    if operation == "shell" {
        return Ok(fixture);
    }
    settle(&mut fixture.app);
    let path = PathBuf::from(if cfg!(windows) {
        "C:/fixture"
    } else {
        "/fixture"
    });
    let state = fixture
        .app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .ok_or_else(|| missing("state"))?;
    let token = state.token();
    let initial = if operation == "batch" {
        items - 128
    } else {
        items
    };
    let data = (0..initial)
        .map(|index| {
            let name = format!("文件_{index:06}.txt");
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
    let snapshot = Arc::new(WidgetryFileDialogSnapshot::prepare(
        token,
        path.clone(),
        data,
        &state.query(),
        None,
    )?);
    WidgetryFileDialog::deliver(
        fixture.app.world_mut(),
        root,
        WidgetryFileDialogReply::Started { token, path },
    )?;
    let selection = fixture
        .app
        .world()
        .get::<WidgetryFileDialogState>(root)
        .ok_or_else(|| missing("state"))?
        .projection_selection_job(snapshot.clone())?
        .prepare()?;
    fixture.reply = Some(WidgetryFileDialogReply::Snapshot {
        snapshot: snapshot.clone(),
        state: WidgetryFileDialogDirectoryState::Ready,
        selection,
    });
    fixture.retained.push(snapshot.clone());
    if operation == "materialize" {
        return Ok(fixture);
    }
    deliver(&mut fixture)?;
    settle(&mut fixture.app);
    validate_text(&mut fixture.app)?;
    if matches!(operation, "batch" | "query") {
        let state = fixture
            .app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .ok_or_else(|| missing("state"))?;
        let snapshot = if operation == "batch" {
            let token = state.token();
            let data = (0..items)
                .map(|index| {
                    let name = format!("文件_{index:06}.txt");
                    WidgetryFileDialogEntryData {
                        path: snapshot.path().join(&name),
                        name: name.into(),
                        kind: WidgetryFileDialogEntryKind::File,
                        size: Some(index as u64),
                        modified: None,
                        hidden: Some(false),
                        system: Some(false),
                    }
                })
                .collect();
            Arc::new(WidgetryFileDialogSnapshot::prepare(
                token,
                snapshot.path().to_owned(),
                data,
                &state.query(),
                Some(&snapshot),
            )?)
        } else {
            WidgetryFileDialog::apply(
                fixture.app.world_mut(),
                root,
                WidgetryFileDialogAction::Search("000".into()),
            )?;
            let state = fixture
                .app
                .world()
                .get::<WidgetryFileDialogState>(root)
                .ok_or_else(|| missing("state"))?;
            Arc::new(snapshot.reproject(state.token(), &state.query())?)
        };
        let selection = fixture
            .app
            .world()
            .get::<WidgetryFileDialogState>(root)
            .ok_or_else(|| missing("state"))?
            .projection_selection_job(snapshot.clone())?
            .prepare()?;
        fixture.reply = Some(WidgetryFileDialogReply::Snapshot {
            snapshot: snapshot.clone(),
            state: WidgetryFileDialogDirectoryState::Ready,
            selection,
        });
        fixture.retained.push(snapshot);
    }
    Ok(fixture)
}

fn deliver(fixture: &mut Fixture) -> Result {
    WidgetryFileDialog::deliver(
        fixture.app.world_mut(),
        fixture.root,
        fixture
            .reply
            .take()
            .ok_or_else(|| missing("prepared reply"))?,
    )?;
    Ok(())
}

fn observe(fixture: &mut Fixture) -> Result<u32> {
    std::hint::black_box(&fixture.assets);
    let world = fixture.app.world_mut();
    let count = world
        .query::<&Name>()
        .iter(world)
        .filter(|name| name.as_str() == "FileDialogEntryRow")
        .count();
    if count > 32 {
        return Err(missing("bounded materialized rows"));
    }
    Ok(world.entities().count_spawned())
}

fn main() -> Result {
    let mut harness = Harness::new("file_dialog-viewport-criterion")?;
    for items in [1_000, 10_000, 100_000] {
        for operation in [
            "shell",
            "materialize",
            "idle",
            "scroll",
            "batch",
            "query",
            "close",
        ] {
            run(
                &mut harness,
                &format!("viewport/n{items}/{operation}"),
                !matches!(operation, "idle" | "scroll"),
                || fixture(items, operation),
                |fixture, index| {
                    match operation {
                        "materialize" | "batch" | "query" => {
                            deliver(fixture)?;
                            settle(&mut fixture.app);
                        }
                        "scroll" => {
                            let area = fixture
                                .app
                                .world_mut()
                                .query::<(Entity, &Name)>()
                                .iter(fixture.app.world())
                                .find(|(_, name)| name.as_str() == "FileDialogEntries")
                                .map(|(entity, _)| entity)
                                .ok_or_else(|| missing("entry area"))?;
                            let viewport = fixture
                                .app
                                .world()
                                .get::<Children>(area)
                                .and_then(|children| {
                                    children.iter().find(|entity| {
                                        fixture
                                            .app
                                            .world()
                                            .get::<WidgetryScrollAreaViewport>(*entity)
                                            .is_some()
                                    })
                                })
                                .ok_or_else(|| missing("viewport"))?;
                            fixture
                                .app
                                .world_mut()
                                .get_mut::<ScrollPosition>(viewport)
                                .ok_or_else(|| missing("scroll"))?
                                .0
                                .y = ((index % 50) * 280) as f32;
                            fixture.app.update();
                        }
                        "close" => {
                            WidgetryFileDialog::apply(
                                fixture.app.world_mut(),
                                fixture.root,
                                WidgetryFileDialogAction::Cancel,
                            )?;
                            fixture.app.world_mut().despawn(fixture.root);
                            fixture.app.update();
                        }
                        _ => fixture.app.update(),
                    }
                    Ok(())
                },
                observe,
            )?;
        }
    }
    harness.finish()
}
