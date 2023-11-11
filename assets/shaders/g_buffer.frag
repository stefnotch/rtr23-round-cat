#version 450

layout (location = 0) in vec3 v_position;
layout (location = 1) in vec3 v_normal;
layout (location = 2) in vec2 v_uv;
layout (location = 3) in vec4 v_tangent;

layout (location = 0) out vec3 outPosition;
layout (location = 1) out vec3 outAlbedo;
layout (location = 2) out vec3 outNormal;

struct DirectionalLight {
    vec3 direction;
    vec3 color;
};

layout(set = 0, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
    vec3 position;
} camera;

layout(set = 1, binding = 0) uniform Material {
    vec3 baseColor;
    vec3 emissivity;
    float roughness;
    float metallic;
} material;

layout(set = 1, binding = 1) uniform sampler2D baseColorTexture;

layout(set = 1, binding = 2) uniform sampler2D normalMapTexture;

void main() {
    vec3 N = normalize(v_normal);
    vec3 T = normalize(v_tangent.xyz);
    vec3 B = cross(v_normal, v_tangent.xyz) * v_tangent.w;
    mat3 TBN = mat3(T,B,N);

    vec3 albedo = texture(baseColorTexture, v_uv).rgb * material.baseColor;

    // in world space
    vec3 norm = TBN * (texture(normalMapTexture, v_uv).rgb * 2.0 - 1.0);

    // in world space
    vec3 n = normalize(norm);

    outPosition = v_position;
    outAlbedo = albedo;
    outNormal = n;
}