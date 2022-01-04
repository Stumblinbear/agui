#version 460

layout(set = 0, binding = 1, r32ui) uniform uimage2D layerMask;

layout (set = 0, binding = 2) uniform texture2D font;
layout (set = 0, binding = 3) uniform sampler s;

layout(location = 0) in vec2 pos;
layout(location = 1) flat in uint layer;
layout(location = 2) in vec4 color;
layout(location = 3) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main() {
    float alpha = texture(sampler2D(font, s), uv).r;

    if (alpha <= 0.0) {
        discard;
    }

    if(imageLoad(layerMask, ivec2(pos.x, pos.y)).x != layer) {
        discard;
    }

    outColor = color * vec4(1.0, 1.0, 1.0, alpha);
}