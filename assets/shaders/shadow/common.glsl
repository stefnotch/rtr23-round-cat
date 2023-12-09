#extension GL_EXT_ray_tracing : require

struct DirectionalLight {
    vec3 direction;
    vec3 color;
    float intensity;
};

layout(set = 0, binding = 0) uniform Scene {
    DirectionalLight directionalLight;
} scene;

layout(set = 1, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
    mat4 view_inv;
    mat4 proj_inv;
    vec3 position;
} camera;

layout (set = 2, binding = 0) uniform accelerationStructureEXT topLevelAS;
layout (set = 2, binding = 1) uniform sampler2D positionBuffer;
layout (set = 2, binding = 2, r8) uniform image2D shadowBuffer;
