#version 450

// 输入：顶点属性
layout(location = 0) in vec2 position;  // 顶点位置
layout(location = 1) in vec3 color;     // 顶点颜色

// 输出：传递给片段着色器的数据
layout(location = 0) out vec3 fragColor;

void main() {
    // 将 2D 位置转换为 4D 齐次坐标
    // z = 0.0 表示在屏幕平面上
    // w = 1.0 表示这是一个位置向量（而非方向向量）
    gl_Position = vec4(position, 0.0, 1.0);
    gl_Position.y = -gl_Position.y; // Vulkan Y 轴翻转

    // 将顶点颜色传递给片段着色器
    // 光栅化阶段会自动在顶点之间插值这个颜色
    fragColor = color;
}