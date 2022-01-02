#version 460

layout (set = 0, binding = 1) uniform texture2D font;
layout (set = 0, binding = 2) uniform sampler s;

layout(location = 0) in vec4 color;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main() {
    float alpha = texture(sampler2D(font, s), uv).r;

    if (alpha <= 0.0) {
        discard;
    }

    outColor = color * vec4(1.0, 1.0, 1.0, alpha);
}