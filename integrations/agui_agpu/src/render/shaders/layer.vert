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

layout(std430, binding = 1) restrict readonly buffer BrushBuffer { vec4 Brushes[]; };
layout(std430, binding = 2) restrict readonly buffer IndexBuffer { uint Indices[]; };
layout(std430, binding = 3) restrict readonly buffer PositionBuffer { vec4 Positions[]; };
layout(std430, binding = 4) restrict readonly buffer TexCoordsBuffer { vec2 TexCoords[]; };

layout(location = 0) in uint brushId;

layout(location = 0) out vec4 outColor;
layout(location = 1) out vec2 outUV;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    uint index = Indices[gl_VertexIndex];

    vec4 pos = Positions[index];
    
    vec2 screen_pos = pos.xy / viewport.size;

    gl_Position = INVERT_Y_AXIS_AND_SCALE * vec4(screen_pos.x, screen_pos.y, 0.0, 1.0);

    vec4 color = Brushes[brushId];

    outColor = color;
    outUV = pos.zw;
}