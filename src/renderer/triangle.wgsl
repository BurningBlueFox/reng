struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

@group(0) @binding(0) var color_texture: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;

@vertex
fn vs_main(
    // Clip space from the CPU, not divided by w.
    @location(0) position: vec4<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = position;
    output.color = color;
    output.uv = uv;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // The sRGB texture samples as linear color; its alpha is ignored.
    let texel = textureSample(color_texture, color_sampler, input.uv);
    return vec4<f32>(input.color * texel.rgb, 1.0);
}
