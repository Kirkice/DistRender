#version 450

// 输入：顶点属性
layout(location = 0) in vec3 position;  // 顶点位置，与 Rust 字段 "position" 对应
layout(location = 1) in vec3 normal;    // 顶点法线，与 Rust 字段 "normal" 对应
layout(location = 2) in vec3 color;     // 顶点颜色，与 Rust 字段 "color" 对应

// Uniform：MVP & Light
layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 projection;
    vec4 lightDir;
    vec4 lightColor;
    vec4 cameraPos;
} ubo;

// 输出传递给片段着色器的数据
layout(location = 0) out vec3 vColor;
layout(location = 1) out vec3 vNormal;
layout(location = 2) out vec3 vFragPos;

void main() {
    vec4 worldPos = ubo.model * vec4(position, 1.0);
    vFragPos = worldPos.xyz;
    vNormal = mat3(ubo.model) * normal;
    vColor = color;
    gl_Position = ubo.projection * ubo.view * worldPos;
}
