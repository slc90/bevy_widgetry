use bevy::{asset::LoadedUntypedAsset, prelude::*};
use bevy_widgetry_core::icon::IconPlugin;

#[derive(Resource)]
struct SvgHandle(Handle<LoadedUntypedAsset>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IconPlugin)
        .add_systems(Startup, load_svg)
        .add_systems(Update, check_loaded)
        .run();
}

fn load_svg(mut commands: Commands, asset_server: Res<AssetServer>) {
    let handle = asset_server.load_builder().load_untyped("icons/check.svg");

    commands.insert_resource(SvgHandle(handle));
}

fn check_loaded(
    handle: Res<SvgHandle>,
    assets: Res<Assets<LoadedUntypedAsset>>,
    mut logged: Local<bool>,
) {
    if !*logged && assets.get(&handle.0).is_some() {
        info!("check.svg loaded successfully");
        *logged = true;
    }
}
