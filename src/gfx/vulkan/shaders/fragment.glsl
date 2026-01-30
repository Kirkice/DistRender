#version 450

// Uniform Buffer
layout(binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 projection;
    vec4 lightDir;      // xyz direction
    vec4 lightColor;    // rgb * intensity
    vec4 cameraPos;
} ubo;

// Fragment Input
layout(location = 0) in vec3 fragPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec3 fragColor;

// Fragment Output
layout(location = 0) out vec4 outColor;

void main() {
    vec3 N = normalize(fragNormal);
    vec3 L = normalize(-ubo.lightDir.xyz);
    vec3 V = normalize(ubo.cameraPos.xyz - fragPos);
    vec3 H = normalize(L + V);

    float diff = max(dot(N, L), 0.0);
    float spec = diff > 0.0 ? pow(max(dot(N, H), 0.0), 32.0) : 0.0;

    vec3 ambient  = 0.1 * ubo.lightColor.rgb;
    vec3 diffuse  = diff * ubo.lightColor.rgb;
    vec3 specular = spec * ubo.lightColor.rgb;

    vec3 finalColor = (ambient + diffuse + specular) * fragColor;
    outColor = vec4(finalColor, 1.0);
}
