#version 460

layout(set = 0, binding = 1, r32ui) uniform uimage2D layerMask;

layout(location = 0) in vec2 pos;
layout(location = 1) flat in uint layer;

layout(location = 0) out uint outLayer;

void main() {
    // Maybe refactor this to directly check for the "parent layer" allowing us to globally increment layer
    // numbers, preventing issues with clipping areas with the same layer bleeding into each other.
    if(imageLoad(layerMask, ivec2(pos.x, pos.y)).x != layer - 1) {
        discard;
    }

    outLayer = layer;
}