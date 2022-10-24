let INVERT_Y_AXIS_AND_SCALE = mat4x4<f32>(
    vec4<f32>(2.0, 0.0, 0.0, 0.0),
    vec4<f32>(0.0, -2.0, 0.0, 0.0),
    vec4<f32>(0.0, 0.0, 1.0, 0.0),
    vec4<f32>(-1.0, 1.0, 0.0, 1.0)
);

let DRAW_TYPE_FONT = 1u;

struct Viewport {
    size: vec2<f32>,
};

struct DrawOptions {
    draw_type: u32,
};

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var<uniform> draw_options: DrawOptions;
@group(0) @binding(2) var<storage, read> indices: array<u32>;
@group(0) @binding(3) var<storage, read> positions: array<vec4<f32>>;
@group(0) @binding(4) var t_texture: texture_2d<f32>;
@group(0) @binding(5) var t_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
) -> VertexOutput {
    let vertex_pos = positions[indices[vertex_index]];

    let screen_pos = (pos + vertex_pos.xy) / viewport.size;

    var result: VertexOutput;

    result.position = INVERT_Y_AXIS_AND_SCALE * vec4<f32>(screen_pos.xy, 0.0, 1.0);
    result.uv = vertex_pos.zw;
    result.color = color;

    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(t_texture, t_sampler, vertex.uv);

    if(draw_options.draw_type == DRAW_TYPE_FONT) {
        color = vec4<f32>(1.0, 1.0, 1.0, color.r);
    }

    if(color.a <= 0.0) {
        discard;
    }

    return color * vertex.color;
}
