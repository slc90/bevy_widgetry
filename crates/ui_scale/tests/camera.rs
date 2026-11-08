#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::{camera::Viewport, prelude::*, render::texture::ManualTextureViews};
use bevy_widgetry_ui_scale::{WidgetryUiScale, WidgetryUiScalePlugin};

const DESIGN: UVec2 = UVec2::new(640, 400);

#[test]
fn shrinking_scale_updates_explicit_viewport_before_render_extraction() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        WindowPlugin::default(),
    ))
    .init_asset::<Image>()
    .init_asset::<Mesh>()
    .init_asset::<bevy::mesh::skinning::SkinnedMeshInverseBindposes>()
    .init_resource::<ManualTextureViews>()
    .add_plugins((
        bevy::camera::CameraPlugin,
        bevy::render::camera::CameraPlugin,
    ));
    let primary = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    {
        let mut window = app.world_mut().get_mut::<Window>(primary).unwrap();
        window.resolution = (DESIGN * 2).into();
        window.resolution.set_scale_factor_override(Some(2.0));
    }
    app.insert_resource(WidgetryUiScale::new(2.0).unwrap())
        .add_plugins(WidgetryUiScalePlugin);
    let camera = app
        .world_mut()
        .spawn((
            Camera2d,
            Camera {
                viewport: Some(Viewport {
                    physical_size: DESIGN * 2,
                    ..default()
                }),
                ..default()
            },
        ))
        .id();
    app.update();
    assert_eq!(
        app.world()
            .get::<Camera>(camera)
            .unwrap()
            .physical_viewport_size(),
        Some(DESIGN * 2)
    );
    app.world_mut()
        .resource_mut::<WidgetryUiScale>()
        .set(1.0)
        .unwrap();
    app.update();
    let window = app.world().get::<Window>(primary).unwrap();
    let camera = app.world().get::<Camera>(camera).unwrap();
    assert_eq!(window.physical_size(), DESIGN);
    assert_eq!(camera.physical_viewport_size(), Some(DESIGN));
    assert_eq!(camera.logical_viewport_size(), Some(DESIGN.as_vec2()));
}
