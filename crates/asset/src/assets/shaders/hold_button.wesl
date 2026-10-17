import bevy_ui_render::ui_vertex_output::UiVertexOutput;

@group(1) @binding(0) var<uniform> background: vec4<f32>;
@group(1) @binding(1) var<uniform> liquid: vec4<f32>;
@group(1) @binding(2) var<uniform> wave: vec4<f32>;
@group(1) @binding(3) var<uniform> border: vec4<f32>;
@group(1) @binding(4) var<uniform> parameters: vec4<f32>;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let radius = length(in.uv - vec2<f32>(0.5));
    let aa = max(fwidth(radius), 0.0001);
    let mask = 1.0 - smoothstep(0.5 - aa, 0.5, radius);
    let inner = 0.5 - parameters.z;
    let progress = parameters.x;
    let time = parameters.y;
    let amplitude = parameters.w * min(1.0, min(progress, 1.0 - progress) * 12.0);
    let baseline = 0.5 + inner * (1.0 - 2.0 * progress);
    let primary = baseline + sin(in.uv.x * 9.0 + time * 1.1) * amplitude;
    let secondary = baseline + sin(in.uv.x * 12.0 - time * 0.85 + 1.6) * amplitude * 0.6;
    let edge = max(fwidth(in.uv.y), 0.0001);
    var fill_main = smoothstep(primary - edge, primary + edge, in.uv.y);
    var fill_wave = smoothstep(secondary - edge, secondary + edge, in.uv.y) * 0.65;
    if progress <= 0.0 { fill_main = 0.0; fill_wave = 0.0; }
    if progress >= 1.0 { fill_main = 1.0; fill_wave = 0.0; }
    let surface = mix(mix(background, wave, fill_wave), liquid, fill_main);
    let ring = smoothstep(inner - aa, inner + aa, radius);
    let color = mix(surface, border, ring);
    return vec4<f32>(color.rgb, color.a * mask);
}
