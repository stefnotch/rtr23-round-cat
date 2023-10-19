#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;

layout (location = 0) out vec3 v_position;
layout (location = 1) out vec3 v_normal;
layout (location = 2) out vec2 v_uv;

struct DirectionalLight {
    vec3 direction;
    vec3 color;
};

layout(set = 0, binding = 0) uniform Scene {
    DirectionalLight directionalLight;
} scene;

layout(set = 1, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
} camera;

layout(set = 2, binding = 0) uniform Material {
    vec3 baseColor;
    vec3 emissivity;
    float roughness;
    float metallic;
} material;

layout(set = 2, binding = 1) uniform sampler2D baseColorTexture;

layout(push_constant) uniform Entity {
    mat4 model;
    mat4 normalMatrix;
} entity;

void main() {
    // in world space
    vec4 worldPos = entity.model * vec4(position, 1.0);

    // in world space
    vec3 n = mat3(entity.normalMatrix) * normal;

    // in clip space
    gl_Position = camera.proj * camera.view * worldPos;

    v_position = position;
    v_normal = normal;
    v_uv = uv;
}
