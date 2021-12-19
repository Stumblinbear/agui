#version 460

layout(location = 0) in vec4 inColor;
layout(location = 0) out vec4 outColor;

// Simply pass the color given by the vertex shader
void main() {
    outColor = inColor;
}