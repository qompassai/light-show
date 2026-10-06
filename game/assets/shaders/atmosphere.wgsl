#import bevy_sprite::mesh2d_vertex_output::VertexOutput

/// Fullscreen background atmosphere: slow-drifting vertical gradient,
/// vignette, and subtle pixel-aligned scanlines for CRT warmth. Drawn
/// once behind everything (low Z); the animation is cheap (a few ALU
/// ops per pixel, no texture reads).
///
/// Material bind group is 2 for 2D materials.
struct AtmosphereUniforms {
    top_color: vec4<f32>,
    bottom_color: vec4<f32>,
    time: f32,
    scanline_strength: f32,
    vignette_strength: f32,
    drift_speed: f32,
};

@group(2) @binding(0) var<uniform> uniforms: AtmosphereUniforms;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv;

    // Slow vertical drift so the background feels alive, not static.
    let drift = sin(uv.y * 3.0 + uniforms.time * uniforms.drift_speed) * 0.04;
    let t = clamp(uv.y + drift, 0.0, 1.0);
    var col = mix(uniforms.bottom_color.rgb, uniforms.top_color.rgb, t);

    // Vignette: darken toward the corners, keeps focus central.
    let vd = length((uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 1.35));
    col *= 1.0 - uniforms.vignette_strength * smoothstep(0.45, 1.0, vd);

    // Scanlines from pixel coordinates (mesh.position is frag coord in
    // pixels): stable across resolutions, no shimmer. One dark line
    // every 4 pixels, strength kept subtle.
    let scan = 1.0 - uniforms.scanline_strength
        * (0.5 + 0.5 * sin(mesh.position.y * 3.14159265 / 4.0));
    col *= scan;

    return vec4<f32>(col, 1.0);
}
