// WGSL Shader for wgpu backend
// 实现 Blinn-Phong 光照模型

// Uniform Buffer Object - MVP 矩阵和光照数据
struct UniformBufferObject {
    model: mat4x4<f32>,
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    light_dir: vec4<f32>,      // xyz: 方向, w: 保留
    light_color: vec4<f32>,    // rgb: 颜色 * 强度, a: 保留
    camera_pos: vec4<f32>,     // xyz: 位置, w: 保留
}

@group(0) @binding(0)
var<uniform> ubo: UniformBufferObject;

// 顶点输入结构
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
}

// 顶点输出 / 片段输入结构
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) frag_pos: vec3<f32>,
    @location(1) frag_normal: vec3<f32>,
    @location(2) frag_color: vec3<f32>,
}

// 顶点着色器
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;

    // 计算世界坐标
    let world_pos = ubo.model * vec4<f32>(input.position, 1.0);
    output.frag_pos = world_pos.xyz;

    // 变换法向量到世界空间（忽略平移）
    output.frag_normal = (ubo.model * vec4<f32>(input.normal, 0.0)).xyz;

    // 传递顶点颜色
    output.frag_color = input.color;

    // 计算裁剪空间坐标 (MVP 变换)
    output.clip_position = ubo.projection * ubo.view * world_pos;

    return output;
}

// 片段着色器 - Blinn-Phong 光照模型
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // 归一化法向量
    let N = normalize(input.frag_normal);

    // 光照方向（从表面指向光源）
    let L = normalize(-ubo.light_dir.xyz);

    // 视线方向（从表面指向相机）
    let V = normalize(ubo.camera_pos.xyz - input.frag_pos);

    // 半向量（Blinn-Phong）
    let H = normalize(L + V);

    // 漫反射系数
    let diff = max(dot(N, L), 0.0);

    // 镜面反射系数（只有在光照可见时计算）
    var spec = 0.0;
    if (diff > 0.0) {
        spec = pow(max(dot(N, H), 0.0), 32.0);  // 32 是高光指数
    }

    // 环境光分量
    let ambient = 0.1 * ubo.light_color.rgb;

    // 漫反射分量
    let diffuse = diff * ubo.light_color.rgb;

    // 镜面反射分量
    let specular = spec * ubo.light_color.rgb;

    // 最终颜色 = (环境光 + 漫反射 + 镜面反射) * 材质颜色
    let final_color = (ambient + diffuse + specular) * input.frag_color;

    return vec4<f32>(final_color, 1.0);
}
