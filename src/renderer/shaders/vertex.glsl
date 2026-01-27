#version 450

// 输入：顶点属性
layout(location = 0) in vec3 position;  // 3D 顶点位置
layout(location = 1) in vec3 color;     // 顶点颜色

// Uniform：MVP 矩阵
layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 model;       // 模型矩阵
    mat4 view;        // 视图矩阵
    mat4 projection;  // 投影矩阵
} ubo;

// 输出：传递给片段着色器的数据
layout(location = 0) out vec3 fragColor;

void main() {
    // 应用 MVP 变换
    gl_Position = ubo.projection * ubo.view * ubo.model * vec4(position, 1.0);

    // 将顶点颜色传递给片段着色器
    // 光栅化阶段会自动在顶点之间插值这个颜色
    fragColor = color;
}