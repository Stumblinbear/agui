#version 460

const mat4 INVERT_Y_AXIS_AND_SCALE = mat4(
    vec4(2.0, 0.0, 0.0, 0.0),
    vec4(0.0, -2.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(-1.0, 1.0, 0.0, 1.0)
);

layout (set = 0, binding = 0) uniform Viewport {
    vec2 size;
} viewport;

layout(location = 0) in vec4 rect;
layout(location = 1) in uint layer;
layout(location = 2) in vec4 uv;
layout(location = 3) in vec4 color;

layout(location = 0) out vec2 outPos;
layout(location = 1) out uint outLayer;
layout(location = 2) out vec4 outColor;
layout(location = 3) out vec2 glyphPos;

void main() {
    uint index = uint[6](0, 2, 1, 1, 2, 3)[gl_VertexIndex];

    vec2[4] verts = vec2[4](
        rect.xy,
        rect.zy,
        rect.xw,
        rect.zw
    );

    vec2 screen_pos = verts[index] / viewport.size;
    
    gl_Position = INVERT_Y_AXIS_AND_SCALE * vec4(screen_pos.x, screen_pos.y, 0.0, 1.0);

    vec2[4] uvs = vec2[4](
        uv.xy,
        uv.zy,
        uv.xw,
        uv.zw
    );

    glyphPos = uvs[index];

    outPos = verts[index];
    outLayer = layer;
    outColor = color;
}