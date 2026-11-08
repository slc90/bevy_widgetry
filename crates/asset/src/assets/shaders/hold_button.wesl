import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> background: vec4<f32>;
@group(1) @binding(1) var<uniform> liquid: vec4<f32>;
@group(1) @binding(2) var<uniform> wave: vec4<f32>;
@group(1) @binding(3) var<uniform> border: vec4<f32>;
@group(1) @binding(4) var<uniform> parameters: vec4<f32>;

const minimum_antialias_width: f32 = 0.0001;
const edge_wave_attenuation: f32 = 12.0;
const primary_wave_frequency: f32 = 9.0;
const primary_wave_speed: f32 = 1.1;
const secondary_wave_frequency: f32 = 12.0;
const secondary_wave_speed: f32 = 0.85;
const secondary_wave_phase: f32 = 1.6;
const secondary_wave_amplitude: f32 = 0.6;
const secondary_wave_opacity: f32 = 0.65;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let radius = length(in.uv - vec2<f32>(0.5));
    let aa = max(fwidth(radius), minimum_antialias_width);
    let mask = 1.0 - smoothstep(0.5 - aa, 0.5, radius);
    let inner = 0.5 - parameters.z;
    let progress = parameters.x;
    let time = parameters.y;
    let amplitude = parameters.w * min(1.0, min(progress, 1.0 - progress) * edge_wave_attenuation);
    let baseline = 0.5 + inner * (1.0 - 2.0 * progress);
    let primary = baseline + sin(in.uv.x * primary_wave_frequency + time * primary_wave_speed) * amplitude;
    let secondary = baseline + sin(in.uv.x * secondary_wave_frequency - time * secondary_wave_speed + secondary_wave_phase) * amplitude * secondary_wave_amplitude;
    let edge = max(fwidth(in.uv.y), minimum_antialias_width);
    var fill_main = smoothstep(primary - edge, primary + edge, in.uv.y);
    var fill_wave = smoothstep(secondary - edge, secondary + edge, in.uv.y) * secondary_wave_opacity;
    if progress <= 0.0 { fill_main = 0.0; fill_wave = 0.0; }
    if progress >= 1.0 { fill_main = 1.0; fill_wave = 0.0; }
    let surface = mix(mix(background, wave, fill_wave), liquid, fill_main);
    let ring = smoothstep(inner - aa, inner + aa, radius);
    let color = mix(surface, border, ring);
    return vec4<f32>(color.rgb, color.a * mask);
}
