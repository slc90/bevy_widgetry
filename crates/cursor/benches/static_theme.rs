use bevy::{
    picking::{
        InteractionPlugin, PickingPlugin,
        backend::{HitData, PointerHits},
        cursor::EntityCursor,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{CursorIcon, CustomCursor, SystemCursorIcon},
};
use bevy_widgetry_cursor::{
    CursorImage, CursorImageSource, CursorSemantic, CursorStyle, WidgetryCursorPlugin,
    WidgetryCursorTheme,
};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run};

struct Fixture {
    app: App,
    window: Entity,
    camera: Entity,
    target: Entity,
    expected: Handle<Image>,
}
fn fixture(windows: usize, depth: usize) -> Result<Fixture> {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        PickingPlugin,
        InteractionPlugin,
        WidgetryCursorPlugin,
    ));
    app.update();
    let window = app.world_mut().spawn(Window::default()).id();
    for _ in 1..windows {
        app.world_mut().spawn(Window::default());
    }
    app.world_mut().spawn(PointerId::Mouse);
    let camera = app.world_mut().spawn_empty().id();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::new_fill(
            Extent3d {
                width: 32,
                height: 24,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255; 4],
            TextureFormat::Rgba8UnormSrgb,
            default(),
        ));
    let theme = WidgetryCursorTheme::new([(
        CursorSemantic::System(SystemCursorIcon::Pointer),
        CursorStyle::Image(CursorImage {
            source: CursorImageSource::Handle(handle.clone()),
            rect: None,
            hotspot: (7, 9),
        }),
    )])
    .map_err(|error| error.into_bevy_error())?;
    WidgetryCursorTheme::set_in_world(app.world_mut(), theme);
    let mut target = app
        .world_mut()
        .spawn(EntityCursor::System(SystemCursorIcon::Pointer))
        .id();
    for _ in 1..depth {
        target = app.world_mut().spawn(ChildOf(target)).id();
    }
    Ok(Fixture {
        app,
        window,
        camera,
        target,
        expected: handle,
    })
}
fn main() -> Result {
    let mut harness = Harness::new("cursor-static-criterion")?;
    for (windows, depth) in [(1, 1), (8, 64)] {
        run(
            &mut harness,
            &format!("cursor-static/w{windows}_d{depth}/mouse_update"),
            false,
            || fixture(windows, depth),
            |fixture, _| {
                let target = bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(
                    fixture.window,
                ))
                .normalize(None)
                .ok_or_else(|| missing("window target"))?;
                fixture.app.world_mut().write_message(PointerInput::new(
                    PointerId::Mouse,
                    Location {
                        target,
                        position: Vec2::ONE,
                    },
                    PointerAction::Move { delta: Vec2::ZERO },
                ));
                fixture.app.world_mut().write_message(PointerHits::new(
                    PointerId::Mouse,
                    vec![(
                        fixture.target,
                        HitData::new(fixture.camera, 0.0, None, None),
                    )],
                    0.0,
                ));
                fixture.app.update();
                let valid = fixture.app.world().get::<CursorIcon>(fixture.window).is_some_and(|cursor| {
                    matches!(cursor, CursorIcon::Custom(CustomCursor::Image(image)) if image.handle == fixture.expected && image.hotspot == (7, 9)
                        && image.texture_atlas.as_ref().and_then(|atlas| atlas.texture_rect(fixture.app.world().resource::<Assets<TextureAtlasLayout>>())) == Some(URect::new(0, 0, 32, 24)))
                });
                if !valid {
                    return Err(missing("requested static cursor"));
                }
                Ok(())
            },
            |fixture| Ok(fixture.app.world().entities().len()),
        )?;
    }
    harness.finish()
}
