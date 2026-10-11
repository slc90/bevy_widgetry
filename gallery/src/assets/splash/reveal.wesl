import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> cover: vec4<f32>;
@group(1) @binding(1) var scene_texture: texture_2d<f32>;
@group(1) @binding(2) var scene_sampler: sampler;
@group(1) @binding(3) var<uniform> viewport_center: vec4<f32>;
@group(1) @binding(4) var<uniform> wave: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = cover.xy + in.uv * cover.zw;
    let scene = textureSample(scene_texture, scene_sampler, uv);
    if wave.w <= 0.0 {
        return vec4<f32>(scene.rgb, 1.0);
    }
    if wave.w >= 1.0 {
        return vec4<f32>(scene.rgb, 0.0);
    }
    let d = length(in.uv * viewport_center.xy - viewport_center.zw);
    let alpha = smoothstep(wave.x - wave.y, wave.x + wave.y, d);
    let ring = 1.0 - smoothstep(0.0, wave.z, abs(d - wave.x));
    let rgb = mix(scene.rgb, vec3<f32>(0.9, 0.86, 1.0), ring * 0.12);
    return vec4<f32>(rgb, alpha);
}
