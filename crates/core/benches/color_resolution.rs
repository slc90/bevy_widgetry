use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_core::{
    foreground::ResolvedForeground, text::WidgetryText, ui::WidgetryUiSystems,
};
use bevy_widgetry_test_utils::{
    benchmark::{
        Harness,
        artifact::{Artifact, error},
        run,
    },
    scene_app,
};
use bevy_widgetry_theme::WidgetryThemeMode;
use std::fs::File;
use std::io::Write;

#[derive(Resource, Default)]
struct Writes {
    text: usize,
    inherited: usize,
}
struct Fixture {
    app: App,
    root: Entity,
    count: usize,
    toggle: bool,
    expected: usize,
}

fn fixture(count: usize, deep: bool, action: &str) -> Result<Fixture> {
    let mut app = scene_app();
    app.init_resource::<Writes>().add_systems(
        PostUpdate,
        (|text: Query<(), (With<WidgetryText>, Changed<TextColor>)>,
          inherited: Query<(), Changed<bevy_widgetry_core::foreground::InheritedForeground>>,
          mut writes: ResMut<Writes>| {
            writes.text = text.iter().count();
            writes.inherited = inherited.iter().count();
        })
        .after(WidgetryUiSystems::ContentColors),
    );
    let root = app.world_mut().spawn(Node::default()).id();
    if action != "theme" {
        app.world_mut()
            .entity_mut(root)
            .insert(ResolvedForeground(Color::WHITE));
    }
    let mut leaf = root;
    for _ in 0..count {
        let parent = if deep { leaf } else { root };
        leaf = app
            .world_mut()
            .spawn((Text::new("content"), WidgetryText, ChildOf(parent)))
            .id();
    }
    for _ in 0..3 {
        app.update();
    }
    Ok(Fixture {
        app,
        root,
        count,
        toggle: false,
        expected: 0,
    })
}
fn operation(f: &mut Fixture, action: &str) -> Result {
    f.toggle = !f.toggle;
    f.expected = if action == "idle" { 0 } else { f.count };
    match action {
        "disabled" => {
            if f.toggle {
                f.app
                    .world_mut()
                    .entity_mut(f.root)
                    .insert(InteractionDisabled);
            } else {
                f.app
                    .world_mut()
                    .entity_mut(f.root)
                    .remove::<InteractionDisabled>();
            }
            f.app
                .world_mut()
                .entity_mut(f.root)
                .insert(ResolvedForeground(if f.toggle {
                    Color::BLACK
                } else {
                    Color::WHITE
                }));
        }
        "foreground" => {
            f.app
                .world_mut()
                .entity_mut(f.root)
                .insert(ResolvedForeground(if f.toggle {
                    Color::BLACK
                } else {
                    Color::WHITE
                }));
        }
        "theme" => {
            WidgetryThemeMode::set_in_world(
                f.app.world_mut(),
                if f.toggle {
                    WidgetryThemeMode::Light
                } else {
                    WidgetryThemeMode::Dark
                },
            )?;
        }
        _ => {}
    }
    f.app.update();
    Ok(())
}
fn verify(f: &mut Fixture, action: &str) -> Result<u32> {
    let writes = f.app.world().resource::<Writes>();
    let inherited = if action == "theme" { 0 } else { f.expected };
    if writes.text != f.expected || writes.inherited != inherited {
        return Err(error(format!(
            "color writes: action={action}, text={}/{}, inherited={}/{inherited}",
            writes.text, f.expected, writes.inherited
        )));
    }
    Ok(f.app.world().entities().count_spawned())
}
fn main() -> Result {
    let mut harness = Harness::new("color-resolution")?;
    let work = Artifact::new("color-resolution-work", "bench; Cargo defaults opt-level=3")?;
    let mut csv = File::create(work.directory.join("writes.csv")).map_err(error)?;
    writeln!(
        csv,
        "content_nodes,shape,operation,text_writes,inherited_writes"
    )
    .map_err(error)?;
    for count in [100, 1000, 10000] {
        for deep in [false, true] {
            let shape = if deep { "deep" } else { "shallow" };
            for action in ["idle", "disabled", "foreground", "theme"] {
                let mut probe = fixture(count, deep, action)?;
                operation(&mut probe, action)?;
                verify(&mut probe, action)?;
                let writes = probe.app.world().resource::<Writes>();
                writeln!(
                    csv,
                    "{count},{shape},{action},{},{}",
                    writes.text, writes.inherited
                )
                .map_err(error)?;
                drop(probe);
                run(
                    &mut harness,
                    &format!("color-resolution/n{count}/{shape}/{action}"),
                    false,
                    || fixture(count, deep, action),
                    |f, _| operation(f, action),
                    |f| verify(f, action),
                )?;
            }
        }
    }
    harness.finish()
}
