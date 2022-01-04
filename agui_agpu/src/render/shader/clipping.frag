#version 460

layout(location = 0) flat in uint layer;

layout(location = 0) out uint outLayer;

void main() {
    outLayer = layer;
}