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
    // Project the UV onto the wipe direction. The projection of the
    // unit square onto dir spans [-extent/2, +extent/2] (extent = the
    // L1 norm of dir), NOT [-0.5, +0.5] — normalizing by extent maps
    // proj to exactly 0..1 for any direction. Without this, a diagonal
    // direction leaves proj < 0 in the starting corner, and the edge
    // math below covers that corner even at progress = 0 (and leaves
    // the far corner uncovered at progress = 1).
    let dir = normalize(
        vec2<f32>(uniforms.direction_x, uniforms.direction_y) + vec2<f32>(1e-5, 0.0)
    );
    let extent = abs(dir.x) + abs(dir.y);
    let proj = dot(in.uv - vec2<f32>(0.5), dir) / extent + 0.5;

    // Covered where proj < progress, with a soft leading edge.
    // progress = 0 -> alpha 0 everywhere; progress = 1 -> alpha 1.
    // The edge travels slightly past both ends (edge = progress *
    // (1 + s)) so the soft band is fully off-screen at progress = 0
    // and fully past the far corner at progress = 1.
    let s = max(uniforms.softness, 1e-4);
    let edge = uniforms.progress * (1.0 + s);
    let alpha = 1.0 - smoothstep(edge - s, edge, proj);

    return vec4<f32>(uniforms.color.rgb, alpha * uniforms.color.a);
}
