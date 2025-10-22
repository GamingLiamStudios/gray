@group(0) @binding(0) var output: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(1)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>
) {
    let index = global_invocation_id.xy;
    let size = textureDimensions(output);

    let pos = vec2f(index.xy) / vec2f(size);
    let color = vec4(pos, 0.0, 1.0);

    textureStore(output, index, color);
}
