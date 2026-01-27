#version 450

// 输入变量
layout(location = 0) in vec3 vColor;      // 可选调试颜色
layout(location = 1) in vec3 vNormal;     // 插值后的法线（世界空间）
layout(location = 2) in vec3 vFragPos;    // 片元世界坐标

// 输出颜色
layout(location = 0) out vec4 f_color;

// 与顶点着色器保持一致的 UBO 定义
layout(set = 0, binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 projection;
    vec4 lightDir;   // xyz 是方向（已归一化，世界空间），w 忽略
    vec4 lightColor; // rgb 颜色 * intensity，a 忽略
    vec4 cameraPos;  // 相机位置 (xyz)
} ubo;

void main() {
    // 归一化法线
    vec3 N = normalize(vNormal);

    // 平行光方向，已确保方向指向片元
    vec3 L = normalize(-ubo.lightDir.xyz);

    // 视线方向
    vec3 V = normalize(ubo.cameraPos.xyz - vFragPos);

    // 半角向量（Blinn）
    vec3 H = normalize(L + V);

    // 漫反射 (Lambert)
    float diff = max(dot(N, L), 0.0);

    // 镜面（Blinn-Phong）
    float spec = 0.0;
    if (diff > 0.0) {
        spec = pow(max(dot(N, H), 0.0), 32.0); // shininess 固定 32
    }

    vec3 ambient = 0.1 * ubo.lightColor.rgb;
    vec3 diffuse = diff * ubo.lightColor.rgb;
    vec3 specular = spec * ubo.lightColor.rgb;

    vec3 finalColor = ambient + diffuse + specular;

    // 可选：混合调试颜色
    finalColor *= vColor;

    f_color = vec4(finalColor, 1.0);
}