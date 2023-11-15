#version 450

layout (set = 0, binding = 0) uniform sampler2D positionBuffer;
layout (set = 0, binding = 1) uniform sampler2D albedoBuffer;
layout (set = 0, binding = 2) uniform sampler2D normalBuffer;
layout (set = 0, binding = 3) uniform sampler2D metallicRoughnessBuffer;

layout (location = 0) in vec2 v_uv;

layout (location = 0) out vec4 fragColor;

struct PointLight {
    vec3 position;
    vec3 color;
    float range;
    float intensity;
};

struct DirectionalLight {
    vec3 direction;
    vec3 color;
};

layout(set = 1, binding = 0) uniform Scene {
    DirectionalLight directionalLight;
} scene;

layout(set = 2, binding = 0) uniform Camera {
    mat4 view;
    mat4 proj;
    vec3 position;
} camera;

const float PI = 3.14159265359;

vec3 ambientLightColor = vec3(1.0, 1.0, 1.0);

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

// n: normalized normal
// l: normalized vector pointing to the light source
// v: normalized view vector pointing to the camera
// h: normalized half-way vector between v and l
float distributionGGXTrowbridgeReitz(vec3 n, vec3 h, float alpha) {
    float alphaSquared = alpha * alpha;

    float nDoth = max(dot(n,h), 0.0);
    float nDothSquared = nDoth * nDoth;

    float partDenom = nDothSquared * (alphaSquared - 1.0) + 1.0;

    return alphaSquared / (PI * partDenom * partDenom);
}

// x: in this context only v or l are allowed to be x
float geometrySchlickBeckmann(vec3 n, vec3 x, float alpha) {
    float k = alpha / 2.0; // there are other options for this
    float nDotx = max(dot(n, x), 0.0);

    return nDotx / max(nDotx * (1.0 - k) + k, 0.000001);
}

float geometrySmith(vec3 n, vec3 v, vec3 l, float alpha) {
    return geometrySchlickBeckmann(n, v, alpha) * geometrySchlickBeckmann(n, l, alpha);
}

vec3 fresnelSchlick(vec3 f0, vec3 v, vec3 h) {
    float vDoth = max(dot(v, h), 0.0);

    return f0 + (1.0 - f0) * pow(1.0 - vDoth, 5.0);
}

vec3 pbr_common(vec3 lightIntensity, vec3 l, vec3 n, vec3 v, vec3 albedo, vec3 f0, float metallic, float roughness) {
    vec3 h = normalize(v + l);

    vec3 fLambert = albedo / PI;

    float alpha = roughness * roughness;

    // D: Normal Distribution Function (GGX/Trowbridge-Reitz)
    float D = distributionGGXTrowbridgeReitz(n, h, alpha);

    // G: Geometry Function (Smith Model using Schlick-Beckmann)
    float G = geometrySmith(n, v, l, alpha);

    // F: Fresnel Function
    vec3 F = fresnelSchlick(f0, v, h);

    vec3 fCookTorranceNumerator = D * G * F;
    float fCookTorranceDenominator = 4.0 * max(dot(n, l), 0.0) * max(dot(n, v), 0.0);
    fCookTorranceDenominator = max(fCookTorranceDenominator, 0.000001);

    vec3 fCookTorrance =  fCookTorranceNumerator / fCookTorranceDenominator;

    vec3 ks = F;
    vec3 kd = vec3(1.0) - ks;
    kd *= 1.0-metallic;

    vec3 diffuseBRDF = kd * fLambert;
    vec3 specularBRDF = /* ks + */ fCookTorrance;
    float nDotL = max(dot(n, l), 0.0);

    return (diffuseBRDF + specularBRDF) * lightIntensity * nDotL;
}

vec3 pbr(PointLight pointLight, vec3 n, vec3 v, vec3 worldPos, vec3 albedo, vec3 f0, float metallic, float roughness) {
    vec3 positionToLight = pointLight.position - worldPos;
    vec3 l = normalize(positionToLight);
    float dSquared = max(dot(positionToLight, positionToLight), 0.000001);

    float attenuation = 1.0 / dSquared;
    vec3 lightIntensity = pointLight.color * pointLight.intensity * attenuation;
    return pbr_common(lightIntensity, l, n, v, albedo, f0, metallic, roughness);
}

vec3 pbr(DirectionalLight directionalLight, vec3 n, vec3 v, vec3 worldPos, vec3 albedo, vec3 f0, float metallic, float roughness) {
    vec3 l = normalize(-directionalLight.direction);

    // TODO: Add directionalLight.intensity

    vec3 lightIntensity = directionalLight.color; /* * directionalLight.intensity; */
    return pbr_common(lightIntensity, l, n, v, albedo, f0, metallic, roughness);
}



void main() {
    vec3 position = texture(positionBuffer, v_uv).rgb;
    vec3 normal = texture(normalBuffer, v_uv).rgb;
    vec3 albedo = texture(albedoBuffer, v_uv).rgb;
    vec2 metallicRoughness = texture(metallicRoughnessBuffer, v_uv).rg;

    float metallic = 0.1;
    float roughness = 0.1;

    // in world space
    vec3 n = normalize(normal);

    // world space
    vec3 v = normalize(camera.position - position); 

    // reflectance at normal incidence (base reflectance)
    // if dia-electric (like plastic) use F0 of 0.04 and if it's a metal, use the albedo as F0 (metallic workflow)
    vec3 f0 = vec3(0.04);
    f0 = mix(f0, albedo, metallic);

    // out going light
    vec3 Lo = vec3(0.0);

    Lo += pbr(scene.directionalLight, n, v, position, albedo, f0, metallic, roughness);

    float ka = 0.03;
    vec3 ambient = (ambientLightColor * ka) * albedo;

    vec3 color = Lo + ambient;

    fragColor = vec4(color, 1.0);
}
