use bevy::core_pipeline::{
    Core2d, Core2dSystems,
    fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin},
    tonemapping::tonemapping,
};
use bevy::ecs::{schedule::ScheduleConfigs, system::BoxedSystem};
use bevy::prelude::*;
use bevy::render::{extract_component::ExtractComponent, render_resource::ShaderType};
use bevy::shader::ShaderRef;
use bevy_widgetry_asset::{BuiltinShader, WidgetryAssetPlugin};

#[derive(Component, Clone, Copy, Default, ExtractComponent, ShaderType)]
#[extract_app(bevy::render::RenderApp)]
pub(crate) struct StraightAlpha {
    padding: Vec4,
}

impl FullscreenMaterial for StraightAlpha {
    fn fragment_shader() -> ShaderRef {
        BuiltinShader::CanvasStraightAlpha.path().into()
    }

    fn schedule() -> impl bevy::ecs::schedule::ScheduleLabel + Clone {
        Core2d
    }

    fn schedule_configs(system: ScheduleConfigs<BoxedSystem>) -> ScheduleConfigs<BoxedSystem> {
        system.in_set(Core2dSystems::PostProcess).after(tonemapping)
    }
}

pub(crate) fn install(app: &mut App) {
    if !app.is_plugin_added::<WidgetryAssetPlugin>() {
        app.add_plugins(WidgetryAssetPlugin);
    }
    app.add_plugins(FullscreenMaterialPlugin::<StraightAlpha>::default());
}
