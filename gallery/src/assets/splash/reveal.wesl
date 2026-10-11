import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> cover: vec4<f32>;
@group(1) @binding(1) var scene_texture: texture_2d<f32>;
@group(1) @binding(2) var scene_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = cover.xy + in.uv * cover.zw;
    let scene = textureSample(scene_texture, scene_sampler, uv);
    return vec4<f32>(scene.rgb, 1.0);
}
