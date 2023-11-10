#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;
layout (location = 2) in vec2 uv;
layout (location = 3) in vec4 tangent;

layout (location = 0) out vec3 v_position;
layout (location = 1) out vec3 v_normal;
layout (location = 2) out vec2 v_uv;
layout (location = 3) out vec4 v_tangent;

struct DirectionalLight {
    vec3 direction;
    vec3 color;
};

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
} camera;

layout(push_constant) uniform Entity {
    mat4 model;
    mat4 normalMatrix;
} entity;

void main() {
    // in world space
    vec4 worldPos = entity.model * vec4(position, 1.0);

    // in world space
    vec3 n = mat3(entity.normalMatrix) * normal;

    gl_Position = camera.proj * camera.view * worldPos;

    v_position = position;
    v_normal = n;
    v_uv = uv;
    v_tangent = tangent;
}