#version 460

layout(binding = 1) uniform DrawOptions {
    uint draw_type;
};

layout(binding = 4) uniform texture2D tex;
layout(binding = 5) uniform sampler textureSampler;

layout(location = 0) in vec4 inColor;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main() {
    vec4 color = texture(sampler2D(tex, textureSampler), uv);

    if(draw_type == 1) {
        color = vec4(1.0, 1.0, 1.0, color.r);
    }

    if(color.a <= 0.0) {
        discard;
    }

    outColor = color * inColor;
}