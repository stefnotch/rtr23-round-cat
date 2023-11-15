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

float Convert_sRGB_ToLinear (float thesRGBValue) {
  return thesRGBValue <= 0.04045f
       ? thesRGBValue / 12.92f
       : pow ((thesRGBValue + 0.055f) / 1.055f, 2.4f);
}

vec3 srgb_to_linear(vec3 v) {
    return vec3(
        Convert_sRGB_ToLinear(v.x),
        Convert_sRGB_ToLinear(v.y),
        Convert_sRGB_ToLinear(v.z)
    );
}

void main() {
    vec3 position = texture(positionBuffer, v_uv).rgb;
    vec3 normal = texture(normalBuffer, v_uv).rgb;
    vec3 albedo = texture(albedoBuffer, v_uv).rgb;

    // in world space
    vec3 n = normalize(normal);

    // in world space
    vec3 l = -normalize(scene.directionalLight.direction);

    float diffuse = max(dot(n, l), 0.05);
    vec3 ambient = vec3(0.01);

    vec3 lightIntensity = scene.directionalLight.color * diffuse + ambient;
    vec3 color = albedo * lightIntensity;


    fragColor = vec4(color, 1.0);
}
