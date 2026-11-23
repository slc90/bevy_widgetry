import bevy_sprite_render::mesh2d::{
    functions::{get_world_from_local, mesh2d_position_local_to_clip},
};

@if(TONEMAP_IN_SHADER)
import bevy_sprite_render::mesh2d::view_bindings::view;
@if(TONEMAP_IN_SHADER)
import bevy_core_pipeline::tonemapping;
@if(SRGB_OUTPUT)
import bevy_render::color_operations::linear_to_srgb;
@if(OKLAB_OUTPUT)
import bevy_render::color_operations::linear_rgb_to_oklab;

struct Gradient {
    origin: vec2<f32>,
    axis: vec2<f32>,
    size: f32,
    radial: u32,
    count: u32,
    offsets: array<vec4<f32>, 2>,
    colors: array<vec4<f32>, 8>,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var<uniform> gradient: Gradient;

struct GradientVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) local: vec2<f32>,
}

@vertex
fn vertex(@builtin(instance_index) instance: u32, @location(0) position: vec3<f32>) -> GradientVertex {
    var out: GradientVertex;
    let model = get_world_from_local(instance);
    out.clip = mesh2d_position_local_to_clip(model, vec4<f32>(position, 1.0));
    out.local = position.xy;
    return out;
}

fn offset_at(index: u32) -> f32 {
    return gradient.offsets[index / 4u][index % 4u];
}

fn gradient_color(t: f32) -> vec4<f32> {
    if t < offset_at(0u) {
        return gradient.colors[0];
    }
    for (var index = 1u; index < gradient.count; index += 1u) {
        let upper = offset_at(index);
        if t < upper {
            let lower = offset_at(index - 1u);
            let proportion = (t - lower) / (upper - lower);
            return mix(gradient.colors[index - 1u], gradient.colors[index], proportion);
        }
    }
    return gradient.colors[gradient.count - 1u];
}

@fragment
fn fragment(input: GradientVertex) -> @location(0) vec4<f32> {
    let local = input.local - gradient.origin;
    var t = dot(local, gradient.axis) / gradient.size;
    if gradient.radial != 0u {
        t = length(local / gradient.size);
    }
    var color = gradient_color(clamp(t, 0.0, 1.0));
@if(TONEMAP_IN_SHADER)
    color = tonemapping::tone_mapping(color, view.color_grading);
@if(SRGB_OUTPUT)
    color = vec4(linear_to_srgb(color.rgb), color.a);
@if(OKLAB_OUTPUT)
    color = vec4(linear_rgb_to_oklab(color.rgb), color.a);
    return color;
}
