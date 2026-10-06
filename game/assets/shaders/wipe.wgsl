#import bevy_ui::ui_vertex_output::UiVertexOutput

/// Fullscreen transition wipe for state changes. `progress` 0 = fully
/// transparent, 1 = fully covered; the wipe sweeps along `direction`
/// with a soft edge of `softness`. Replaces the flat alpha fade with a
/// directional sweep (dream-transition feel).
///
/// UI materials bind at group 1 (group 0 is view + globals).
struct WipeUniforms {
    color: vec4<f32>,
    progress: f32,
    softness: f32,
    direction_x: f32,
    direction_y: f32,
};

@group(1) @binding(0) var<uniform> uniforms: WipeUniforms;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    // Project the UV onto the wipe direction, normalized to 0..1.
    let dir = normalize(
        vec2<f32>(uniforms.direction_x, uniforms.direction_y) + vec2<f32>(1e-5, 0.0)
    );
    let proj = dot(in.uv - vec2<f32>(0.5), dir) + 0.5;

    // Covered where proj < progress, with a soft leading edge.
    // progress = 0 -> alpha 0 everywhere; progress = 1 -> alpha 1.
    let s = max(uniforms.softness, 1e-4);
    let alpha = 1.0 - smoothstep(uniforms.progress - s, uniforms.progress, proj);

    return vec4<f32>(uniforms.color.rgb, alpha * uniforms.color.a);
}
