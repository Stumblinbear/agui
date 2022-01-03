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

layout(location = 0) in float depth;
layout(location = 1) in vec2 pos;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    vec2 pos = pos / viewport.size;
     
    gl_Position = INVERT_Y_AXIS_AND_SCALE * vec4(pos.x, pos.y, depth, 1.0);
}