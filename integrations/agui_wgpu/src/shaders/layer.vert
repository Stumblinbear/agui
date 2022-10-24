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

layout(binding = 1) uniform DrawOptions {
    uint draw_type;
};

layout(std430, binding = 2) restrict readonly buffer IndexBuffer { uint Indices[]; };
layout(std430, binding = 3) restrict readonly buffer PositionBuffer { vec4 Positions[]; };

layout(location = 0) in vec2 pos;
layout(location = 1) in vec4 color;

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec2 outUV;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    uint index = Indices[gl_VertexIndex];

    vec4 vertex_pos = Positions[index];
    
    vec2 screen_pos = (pos + vertex_pos.xy) / viewport.size;

    gl_Position = INVERT_Y_AXIS_AND_SCALE * vec4(screen_pos.x, screen_pos.y, 0.0, 1.0);

    outColor = color;
    outUV = vertex_pos.zw;
}