

#version 460

layout(location = 0) in vec4 rect;
layout(location = 1) in vec4 color;

// Workaround Naga validation
out gl_PerVertex {
    vec4 gl_Position;
};

// Draws a rectangle with 6 vertices
// PERF: I'm sure there is some math magic that can make this much faster
void main() {
    vec2[4] verts = vec2[4](
        rect.xy,
        rect.xy + vec2(rect.z, 0.0),
        rect.xy + vec2(0.0, rect.w),
        rect.xy + rect.zw
    );

    // Draw a rectangle with two triangles.
    // A(0)          B(1)
    //    0  --- 2,3
    //    |   /   |
    //   1,4 ---  5
    // C(2)          D(3)
    uint index = uint[6](0, 2, 1, 1, 2, 3)[gl_VertexIndex];
    vec2 pos = verts[index] / vec2(800.0, 600.0);
     
    gl_Position = vec4(pos.x, pos.y, 0.0, 1.0);
    // Adjust to texture coord style
    gl_Position.y = 1 - gl_Position.y;
    gl_Position.x -= 1;
}