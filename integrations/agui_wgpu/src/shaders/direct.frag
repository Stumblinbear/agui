#version 460

layout(binding = 1) uniform DrawOptions {
    uint draw_type;
};

layout(binding = 3) uniform texture2D tex;
layout(binding = 4) uniform sampler textureSampler;

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 color = texture(sampler2D(tex, textureSampler), uv);

    if(color.a <= 0.0) {
        discard;
    }

    outColor = color;
}