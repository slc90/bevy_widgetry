use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinFont;
use bevy_widgetry_canvas::{CanvasDocument, CanvasTextStyle, CanvasView};
use bevy_widgetry_test_utils::benchmark::{Harness, artifact::error, run};
use bevy_widgetry_test_utils::{ErrorCapture, add_ui_plugins, scene_app, spawn_ui_camera};
use skrifa::{FontRef, MetadataProvider};
use std::collections::HashSet;

const LABELS: usize = 32;
const GLYPHS_PER_LABEL: usize = 16;
const ZOOM_STEPS: usize = 32;
const VIEWPORT: UVec2 = UVec2::new(1400, 900);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MeshUsage {
    count: usize,
    geometry_bytes: usize,
}

struct Fixture {
    app: App,
    main: Entity,
    baseline: HashSet<AssetId<Mesh>>,
    warmed: Option<MeshUsage>,
}

fn frame(app: &mut App) -> Result {
    let errors = ErrorCapture::default();
    errors.run(|| app.update());
    let errors = errors.take();
    if !errors.is_empty() {
        return Err(error(format!("Canvas text frame errors: {errors:?}")));
    }
    Ok(())
}

fn fixture(labels: &[String]) -> Result<Fixture> {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, bevy_widgetry_canvas::CanvasPlugin));
    app.set_error_handler(ErrorCapture::handler());
    let camera = spawn_ui_camera(&mut app, VIEWPORT, 1.0);
    let document = app.world_mut().spawn(CanvasDocument).id();
    let mut view = || {
        app.world_mut()
            .spawn_scene(bsn! {
                @CanvasView { @document:{Some(document)} }
                template(move |_| Ok(UiTargetCamera(camera)))
                Node { width:px(VIEWPORT.x / 2), height:px(VIEWPORT.y) }
            })
            .map(|view| view.id())
            .map_err(BevyError::error)
    };
    let main = view()?;
    view()?;
    frame(&mut app)?;
    frame(&mut app)?;
    let baseline = app.world().resource::<Assets<Mesh>>().ids().collect();
    for (row, text) in labels.iter().enumerate() {
        CanvasDocument::create_text(
            app.world_mut(),
            document,
            CanvasTextStyle {
                content: text.clone(),
                ..default()
            },
            Transform::from_xyz(0.0, (row as f32 - LABELS as f32 * 0.5) * 26.0, 0.0),
        )
        .map_err(|error| error.into_bevy_error())?;
    }
    Ok(Fixture {
        app,
        main,
        baseline,
        warmed: None,
    })
}

fn distinct_labels() -> Result<Vec<String>> {
    let font = FontRef::new(BuiltinFont::Default.bytes()).map_err(error)?;
    let mut seen = HashSet::new();
    let characters: Vec<_> = font
        .charmap()
        .mappings()
        .filter(|(codepoint, id)| (0x4e00..=0x9fff).contains(codepoint) && seen.insert(*id))
        .filter_map(|(codepoint, _)| char::from_u32(codepoint))
        .take(LABELS * GLYPHS_PER_LABEL)
        .collect();
    if characters.len() != LABELS * GLYPHS_PER_LABEL {
        return Err(error("the builtin font has too few distinct CJK glyphs"));
    }
    Ok(characters
        .as_chunks::<GLYPHS_PER_LABEL>()
        .0
        .iter()
        .map(|label| label.iter().collect())
        .collect())
}

impl Fixture {
    fn assets(&self) -> MeshUsage {
        let mut usage = MeshUsage {
            count: 0,
            geometry_bytes: 0,
        };
        for (id, mesh) in self.app.world().resource::<Assets<Mesh>>().iter() {
            if !self.baseline.contains(&id) {
                usage.count += 1;
                usage.geometry_bytes += mesh.count_vertices() * size_of::<[f32; 3]>()
                    + mesh.indices().map_or(0, |indices| indices.len()) * size_of::<u32>();
            }
        }
        usage
    }

    fn zoom(&mut self, step: usize) -> Result {
        let step = ZOOM_STEPS.abs_diff(step % (2 * ZOOM_STEPS));
        let zoom = 2.0_f32.powf(-1.0 + 4.0 * step as f32 / ZOOM_STEPS as f32);
        CanvasView::set_zoom_in_world(self.app.world_mut(), self.main, zoom)
            .map_err(|error| error.into_bevy_error())?;
        frame(&mut self.app)
    }

    fn warm(&mut self) -> Result {
        for step in 0..=ZOOM_STEPS {
            self.zoom(step)?;
        }
        self.zoom(1)?;
        self.warmed = Some(self.assets());
        Ok(())
    }

    fn observe(&self) -> Result<u32> {
        if let Some(expected) = self.warmed
            && self.assets() != expected
        {
            return Err(error("warm zoom added or removed glyph geometry"));
        }
        Ok(self.app.world().entities().count_spawned())
    }
}

fn main() -> Result {
    let mut harness = Harness::new("canvas-text-criterion")?;
    let repeated = vec!["O回".repeat(GLYPHS_PER_LABEL / 2); LABELS];
    let distinct = distinct_labels()?;
    for (name, labels, expected) in [
        ("repeated", repeated, 2),
        ("cjk", distinct, LABELS * GLYPHS_PER_LABEL),
    ] {
        let mut sample = fixture(&labels)?;
        frame(&mut sample.app)?;
        let cold = sample.assets();
        if cold.count != expected {
            return Err(error(format!(
                "unexpected cold glyph workload {name}: {cold:?}"
            )));
        }
        sample.warm()?;
        println!(
            "views=2 scenario={name} glyphs_per_view={} cold={cold:?} warm={:?}",
            LABELS * GLYPHS_PER_LABEL,
            sample.assets()
        );
        run(
            &mut harness,
            &format!("canvas_text/views2/{name}/first_projection"),
            true,
            || fixture(&labels),
            |fixture, _| frame(&mut fixture.app),
            |fixture| fixture.observe(),
        )?;
        run(
            &mut harness,
            &format!("canvas_text/views2/{name}/cached_zoom"),
            false,
            || {
                let mut fixture = fixture(&labels)?;
                fixture.warm()?;
                Ok(fixture)
            },
            |fixture, step| fixture.zoom(step),
            |fixture| fixture.observe(),
        )?;
    }
    harness.finish()
}
