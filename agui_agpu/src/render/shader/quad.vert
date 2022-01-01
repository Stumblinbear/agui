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
layout(location = 1) in float z;
layout(location = 2) in vec4 color;
layout(location = 3) in float radius;

layout(location = 0) out vec4 outColor;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    vec2[4] verts = vec2[4](
        rect.xy,
        rect.xy + vec2(rect.z, 0.0),
        rect.xy + vec2(0.0, rect.w),
        rect.xy + rect.zw
    );

    uint index = uint[6](0, 2, 1, 1, 2, 3)[gl_VertexIndex];
    vec2 pos = verts[index] / viewport.size;
     
    gl_Position = INVERT_Y_AXIS_AND_SCALE * vec4(pos.x, pos.y, z, 1.0);
    
    outColor = color;
}