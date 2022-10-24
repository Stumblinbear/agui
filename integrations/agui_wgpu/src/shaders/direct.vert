#version 460

const mat4 INVERT_Y_AXIS_AND_SCALE = mat4(
    vec4(2.0, 0.0, 0.0, 0.0),
    vec4(0.0, -2.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(-1.0, 1.0, 0.0, 1.0)
);

layout (binding = 0) uniform Viewport {
    vec2 size;
} viewport;

layout(std430, binding = 1) restrict readonly buffer IndexBuffer { uint Indices[]; };
layout(std430, binding = 2) restrict readonly buffer PositionBuffer { vec4 Positions[]; };

layout(location = 0) in vec2 pos;

layout(location = 0) out vec2 outUV;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    uint index = Indices[gl_VertexIndex];

    vec4 vertex_pos = Positions[index];

    vec2 screen_pos = (pos + vertex_pos.xy) / viewport.size;

    gl_Position = INVERT_Y_AXIS_AND_SCALE * vec4(screen_pos.x, screen_pos.y, 0.0, 1.0);

    outUV = vertex_pos.zw;
}
