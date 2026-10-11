use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub(crate) struct SplashRevealMaterial {
    #[uniform(0)]
    pub(super) cover: Vec4,
    #[texture(1)]
    #[sampler(2)]
    pub(super) scene: Handle<Image>,
}

impl UiMaterial for SplashRevealMaterial {
    fn fragment_shader() -> ShaderRef {
        crate::assets::splash_shader().into()
    }
}
