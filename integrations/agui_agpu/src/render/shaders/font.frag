#version 460

layout(binding = 4) uniform texture2D tex;
layout(binding = 5) uniform sampler textureSampler;

layout(location = 0) in vec4 inColor;
layout(location = 1) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main() {
    float alpha = texture(sampler2D(tex, textureSampler), uv).r;

    if(alpha <= 0.0) {
        discard;
    }

    outColor = vec4(1.0, 1.0, 1.0, alpha) * inColor;
}