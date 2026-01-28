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

// Vertex Input
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec3 color;

// Vertex Output
layout(location = 0) out vec3 fragPos;
layout(location = 1) out vec3 fragNormal;
layout(location = 2) out vec3 fragColor;

void main() {
    vec4 worldPos = ubo.model * vec4(position, 1.0);
    fragPos = worldPos.xyz;
    fragNormal = mat3(ubo.model) * normal;
    fragColor = color;
    gl_Position = ubo.projection * ubo.view * worldPos;
}
