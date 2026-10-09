use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardInput, NativeKey},
};
use bevy::prelude::*;
use bevy_widgetry_file_dialog::*;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, owned_widgetry_window,
    widgetry_window_target,
};

struct Fixture {
    app: App,
    roots: Vec<Entity>,
    native: Entity,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.insert_resource(WidgetryFileDialogRuntimeOptions {
        automatic: false,
        ..default()
    });
    app.add_plugins(WidgetryFileDialogPlugin);
    let parent=app.world_mut().spawn_scene(bsn! {owned_widgetry_window(Window::default(),WidgetryWindowControlsConfig::default(),WidgetryWindowBackground::Theme, Default::default(), bsn_list![],bsn_list![Text("Parent window") bevy_widgetry_core::text::WidgetryText])})?.id();
    settle(&mut app);
    let parent_native =
        widgetry_window_target(app.world(), parent).ok_or_else(|| missing("parent window"))?;
    let mut roots = Vec::new();
    for _ in 0..count {
        roots.push(app.world_mut().spawn_scene(bsn! { @WidgetryFileDialog { @window:{Some(WidgetryFileDialogWindow {parent:Some(parent_native),modality:WidgetryFileDialogModality::Modal,..default()})} } })?.id());
    }
    settle(&mut app);
    // Headless fixture 提供真实窗口尺寸，测量 ECS/layout，不包含 GPU 或 native presentation。
    for mut camera in app
        .world_mut()
        .query::<&mut Camera>()
        .iter_mut(app.world_mut())
    {
        camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: UVec2::new(1000, 700),
            scale_factor: 1.0,
        });
    }
    settle(&mut app);
    validate_text(&mut app)?;
    let native = roots
        .last()
        .and_then(|root| widgetry_window_target(app.world(), *root))
        .ok_or_else(|| missing("dialog window"))?;
    Ok(Fixture { app, roots, native })
}

fn main() -> Result {
    let mut harness = Harness::new("file_dialog-window-criterion")?;
    for count in [1, 4, 8] {
        for operation in ["idle", "tab", "close"] {
            run(
                &mut harness,
                &format!("owned/n{count}/{operation}"),
                operation == "close",
                || fixture(count),
                |fixture, _| {
                    if operation == "tab" {
                        fixture.app.world_mut().write_message(KeyboardInput {
                            window: fixture.native,
                            key_code: KeyCode::Tab,
                            logical_key: Key::Unidentified(NativeKey::Unidentified),
                            state: ButtonState::Pressed,
                            text: None,
                            repeat: false,
                        });
                    } else if operation == "close" {
                        for root in &fixture.roots {
                            WidgetryFileDialog::apply(
                                fixture.app.world_mut(),
                                *root,
                                WidgetryFileDialogAction::Cancel,
                            )?;
                        }
                    }
                    fixture.app.update();
                    if operation == "close"
                        && fixture
                            .roots
                            .iter()
                            .any(|root| fixture.app.world().get_entity(*root).is_ok())
                    {
                        return Err(missing("owned cleanup"));
                    }
                    Ok(())
                },
                |fixture| Ok(fixture.app.world().entities().count_spawned()),
            )?;
        }
    }
    harness.finish()
}
