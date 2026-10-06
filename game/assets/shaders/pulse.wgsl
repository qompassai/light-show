#import bevy_sprite::mesh2d_vertex_output::VertexOutput

/// Light pulse traveling along a fiber. The quad's U axis runs along the
/// fiber direction; a gaussian-brightness head loops from 0 to 1 at
/// `speed` pulses per second. Replaces frame-based sprite pulses with a
/// smooth, resolution-independent flow.
///
/// Material bind group is 2 for 2D materials.
struct PulseUniforms {
    color: vec4<f32>,
    time: f32,
    speed: f32,
    width: f32,
    base_brightness: f32,
};

@group(2) @binding(0) var<uniform> uniforms: PulseUniforms;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    // Looping head position along the fiber.
    let head = fract(uniforms.time * uniforms.speed);

    // Wrapped distance so the pulse flows continuously, no popping.
    var d = abs(mesh.uv.x - head);
    d = min(d, 1.0 - d);

    // Gaussian falloff around the head. exp() is portable across
    // Vulkan/Metal/D3D12/GLES3.
    let w = max(uniforms.width, 1e-4);
    let pulse = exp(-d * d / (w * w));

    // Soft vertical falloff so the quad edges don't show.
    let v = sin(mesh.uv.y * 3.14159265);
    let brightness = uniforms.base_brightness + pulse;

    let alpha = clamp(v * (uniforms.base_brightness + pulse), 0.0, 1.0);
    return vec4<f32>(uniforms.color.rgb * brightness * v, alpha * uniforms.color.a);
}
