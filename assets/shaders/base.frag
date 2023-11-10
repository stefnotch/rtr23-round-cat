#version 450

layout (set = 0, binding = 0) uniform sampler2D positionBuffer;
layout (set = 0, binding = 1) uniform sampler2D albedoBuffer;
layout (set = 0, binding = 2) uniform sampler2D normalBuffer;

layout (location = 0) in vec2 v_uv;

layout (location = 0) out vec4 fragColor;

struct DirectionalLight {
    vec3 direction;
    vec3 color;
};

layout(set = 1, binding = 0) uniform Scene {
    DirectionalLight directionalLight;
} scene;

void main() {
    vec3 position = texture(positionBuffer, v_uv).rgb;
    vec3 normal = texture(normalBuffer, v_uv).rgb;
    vec3 albedo = texture(albedoBuffer, v_uv).rgb;

    // in world space
    vec3 n = normalize(normal);

    // in world space
    vec3 l = -normalize(scene.directionalLight.direction);

    float diffuse = max(dot(n, l), 0.05);

    vec3 lightIntensity = scene.directionalLight.color * diffuse;

    fragColor = vec4(albedo * lightIntensity, 1.0);
}