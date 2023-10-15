#version 450

layout (location = 0) in vec3 v_position;
layout (location = 1) in vec3 v_normal;
layout (location = 2) in vec2 v_uv;

layout (location = 0) out vec4 fragColor;

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
    vec3 position;
} camera;

layout(set = 2, binding = 0) uniform Material {
    vec3 baseColor;
    vec3 emissivity;
    float roughness;
    float metallic;
} material;

layout(set = 2, binding = 1) uniform sampler2D baseColorTexture;
void main() {
    // in world space
    vec3 worldPos = v_position;

    // in world_space
    vec3 n = normalize(v_normal);

    // in world space
    vec3 l = -normalize(scene.directionalLight.direction);

    float diffuse = max(dot(n, l), 0.1);

    vec3 lightIntensity = scene.directionalLight.color * diffuse;

    fragColor = vec4(lightIntensity, 1.0);
}