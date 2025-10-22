struct VertexOut {
    @location(0) texture: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var sample: sampler;

var<private> v_positions: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(1.0, -1.0),  // TL
    vec2<f32>(1.0, 1.0),   // TR
    vec2<f32>(-1.0, -1.0), // BL
    vec2<f32>(-1.0, 1.0),  // BR
);

var<private> v_texcoord: array<vec2<f32>, 4> = array<vec2<f32>, 4>(
    vec2<f32>(0.0, 0.0), // TL
    vec2<f32>(0.0, 1.0), // TR
    vec2<f32>(1.0, 0.0), // BL
    vec2<f32>(1.0, 1.0), // BR
);

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOut {
    var out: VertexOut;

    out.position = vec4<f32>(v_positions[v_idx], 0.0, 1.0);
    out.texture = v_texcoord[v_idx];

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(texture, sample, in.texture);
}