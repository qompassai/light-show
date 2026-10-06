#import bevy_sprite::mesh2d_vertex_output::VertexOutput

/// Radial glow with a breathing pulse. Rendered on a quad; the glow is
/// strongest at the center and falls off quadratically. Used for win
/// bursts, node highlights, and any "light bloom" moment.
///
/// Material bind group is 2 for 2D materials (0 = view, 1 = mesh).
struct GlowUniforms {
    color: vec4<f32>,
    time: f32,
    intensity: f32,
    pulse_speed: f32,
    pulse_amount: f32,
};

@group(2) @binding(0) var<uniform> uniforms: GlowUniforms;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Center the UVs: 0 at quad center, 1 at the edge.
    let centered = (mesh.uv - vec2<f32>(0.5)) * 2.0;
    let dist = length(centered);

    // Breathing pulse: intensity oscillates around the base value.
    let pulse = 1.0 + uniforms.pulse_amount * sin(uniforms.time * uniforms.pulse_speed);

    // Quadratic falloff — hot core, soft edge. Portable: no texture reads.
    let glow = pow(max(0.0, 1.0 - dist), 2.0) * uniforms.intensity * pulse;

    let alpha = clamp(glow, 0.0, 1.0) * uniforms.color.a;
    // Premultiplied-style output: rgb scaled by glow so additive-feel
    // blending blooms on bright backgrounds too.
    return vec4<f32>(uniforms.color.rgb * glow, alpha);
}
