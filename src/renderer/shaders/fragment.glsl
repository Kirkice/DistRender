#version 450

// 输入：从顶点着色器传来的插值颜色
layout(location = 0) in vec3 fragColor;

// 输出：最终的像素颜色
layout(location = 0) out vec4 f_color;

void main() {
    // 将 RGB 颜色扩展为 RGBA
    // alpha = 1.0 表示完全不透明
    f_color = vec4(fragColor, 1.0);
}