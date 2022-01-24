#version 460

layout (set = 0, binding = 5) uniform texture2D tex;
layout (set = 0, binding = 6) uniform sampler textureSampler;

layout(location = 0) in vec4 inColor;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 color = texture(sampler2D(tex, textureSampler), uv) * inColor;

    if (color.a <= 0.0) {
        discard;
    }

    outColor = color;
}