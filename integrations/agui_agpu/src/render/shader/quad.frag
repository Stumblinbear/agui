#version 460

layout(set = 0, binding = 1, r32ui) uniform uimage2D layerMask;

layout(location = 0) in vec2 pos;
layout(location = 1) flat in uint layer;
layout(location = 2) in vec4 inColor;

layout(location = 0) out vec4 outColor;

void main() {
    if(imageLoad(layerMask, ivec2(pos.x, pos.y)).x != layer) {
        discard;
    }

    outColor = inColor;
}