# 相机系统集成指南

本文档说明如何在渲染器中使用场景配置文件（scene.toml）中的相机和 MVP 变换。

## 已完成的工作 ✅

### 1. 场景配置系统
- ✅ 创建了 `SceneConfig` 结构（[src/core/scene.rs](src/core/scene.rs)）
- ✅ 支持相机配置（位置、旋转、FOV、裁剪面）
- ✅ 支持模型变换（位置、旋转、缩放）
- ✅ 创建了示例配置文件 [scene.toml](scene.toml)

### 2. 数学支持
- ✅ Transform 结构可以生成模型矩阵 (`to_matrix()`)
- ✅ CameraConfig 可以生成视图矩阵 (`view_matrix()`)
- ✅ CameraConfig 可以生成投影矩阵 (`projection_matrix(aspect_ratio)`)

### 3. 着色器更新
- ✅ Vulkan 顶点着色器支持 3D 位置和 MVP 矩阵（[src/renderer/shaders/vertex.glsl](src/renderer/shaders/vertex.glsl)）
- ✅ DX12 着色器支持 3D 位置和 MVP 矩阵（[src/renderer/shaders/shader.hlsl](src/renderer/shaders/shader.hlsl)）
- ✅ UniformBufferObject 定义（包含 model, view, projection 矩阵）

### 4. 顶点结构
- ✅ MyVertex 更新为 3D（position: [f32; 3]，24 字节）
- ✅ GeometryVertex 转换函数更新

## 需要完成的集成步骤

### 步骤 1: 在主程序中加载场景配置

修改 [src/main.rs](src/main.rs)：

```rust
use core::{Config, SceneConfig, log};

fn main() {
    // ... 现有的配置加载代码 ...

    // 加载场景配置
    let scene = SceneConfig::from_file_or_default("scene.toml");

    // 创建渲染器（传递场景配置）
    let mut renderer = match Renderer::new(&event_loop, &config, &scene) {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to initialize renderer: {}", e);
            eprintln!("Failed to initialize renderer: {}", e);
            std::process::exit(1);
        }
    };

    // ... 其余代码 ...
}
```

### 步骤 2: Vulkan 渲染器集成

需要在 [src/renderer/vulkan.rs](src/renderer/vulkan.rs) 中添加：

```rust
// 1. 添加导入
use vulkano::buffer::CpuBufferPool;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::pipeline::{Pipeline, PipelineBindPoint};
use bytemuck::{Pod, Zeroable};
use crate::core::{SceneConfig, Matrix4};

// 2. 定义 UBO 结构
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
struct UniformBufferObject {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

// 3. 在 Renderer 结构中添加字段
pub struct Renderer {
    // ... 现有字段 ...
    uniform_buffer_pool: CpuBufferPool<UniformBufferObject>,
    scene: SceneConfig,
}

// 4. 在 new() 中初始化
impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &SceneConfig) -> Result<Self> {
        // ... 现有代码 ...

        // 创建 uniform buffer pool
        let uniform_buffer_pool = CpuBufferPool::uniform_buffer(gfx.device.clone());

        Ok(Self {
            // ... 现有字段 ...
            uniform_buffer_pool,
            scene: scene.clone(),
        })
    }

    pub fn draw(&mut self) -> Result<()> {
        // 计算 MVP 矩阵
        let aspect_ratio = self.viewport.dimensions[0] / self.viewport.dimensions[1];

        let model = self.scene.model.transform.to_matrix();
        let view = self.scene.camera.view_matrix();
        let projection = self.scene.camera.projection_matrix(aspect_ratio);

        let ubo = UniformBufferObject {
            model: *model.as_ref(),
            view: *view.as_ref(),
            projection: *projection.as_ref(),
        };

        // 分配 uniform buffer
        let uniform_subbuffer = self.uniform_buffer_pool.from_data(ubo)
            .map_err(|e| DistRenderError::Graphics(
                GraphicsError::ResourceCreation(format!("Failed to create uniform buffer: {:?}", e))
            ))?;

        // 创建描述符集
        let layout = self.pipeline.layout().set_layouts().get(0).unwrap();
        let descriptor_set = PersistentDescriptorSet::new(
            &self.gfx.descriptor_set_allocator,
            layout.clone(),
            [WriteDescriptorSet::buffer(0, uniform_subbuffer)],
        )
        .map_err(|e| DistRenderError::Graphics(
            GraphicsError::ResourceCreation(format!("Failed to create descriptor set: {:?}", e))
        ))?;

        // 在命令缓冲区中绑定描述符集
        builder
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                descriptor_set,
            )
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)?;

        // ... 其余绘制代码 ...
    }
}
```

### 步骤 3: DX12 渲染器集成

需要在 [src/renderer/dx12.rs](src/renderer/dx12.rs) 中：

```rust
// 1. 更新输入布局（vec3 position）
let input_element_descs = [
    D3D12_INPUT_ELEMENT_DESC {
        SemanticName: windows::core::s!("POSITION"),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,  // vec3 而不是 vec2
        InputSlot: 0,
        AlignedByteOffset: 0,
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
    D3D12_INPUT_ELEMENT_DESC {
        SemanticName: windows::core::s!("COLOR"),
        SemanticIndex: 0,
        Format: DXGI_FORMAT_R32G32B32_FLOAT,
        InputSlot: 0,
        AlignedByteOffset: 12,  // 更新偏移量（3 * 4 = 12）
        InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        InstanceDataStepRate: 0,
    },
];

// 2. 创建常量缓冲区
#[repr(C, align(256))]  // D3D12 要求 256 字节对齐
struct UniformBufferObject {
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

// 3. 在渲染循环中更新常量缓冲区
// 计算 MVP
let model = self.scene.model.transform.to_matrix();
let view = self.scene.camera.view_matrix();
let projection = self.scene.camera.projection_matrix(aspect_ratio);

let ubo = UniformBufferObject {
    model: *model.as_ref(),
    view: *view.as_ref(),
    projection: *projection.as_ref(),
};

// 上传到常量缓冲区
// ... 使用 Map/Unmap 更新数据 ...
```

## 使用场景配置文件

编辑 [scene.toml](scene.toml) 来调整相机和模型：

```toml
[camera]
[camera.transform]
position = [0.0, 0.0, 3.0]  # 相机在 Z 轴 3 单位远处
rotation = [0.0, 0.0, 0.0]  # 向前看
scale = [1.0, 1.0, 1.0]

fov = 60.0        # 60 度视野
near_clip = 0.1   # 近裁剪面
far_clip = 100.0  # 远裁剪面

[model]
path = "assets/models/sphere.obj"

[model.transform]
position = [0.0, 0.0, 0.0]  # 模型在原点
rotation = [0.0, 0.0, 0.0]  # 无旋转
scale = [1.0, 1.0, 1.0]     # 原始大小
```

## 测试不同的相机角度

### 示例 1: 从上方观察
```toml
[camera.transform]
position = [0.0, 2.0, 2.0]  # 上方位置
rotation = [-45.0, 0.0, 0.0]  # 向下看 45 度
```

### 示例 2: 侧面视角
```toml
[camera.transform]
position = [3.0, 0.0, 3.0]
rotation = [0.0, -45.0, 0.0]  # 向中心旋转
```

### 示例 3: 旋转模型
```toml
[model.transform]
position = [0.0, 0.0, 0.0]
rotation = [0.0, 45.0, 0.0]  # Y 轴旋转 45 度
scale = [1.5, 1.5, 1.5]      # 放大 1.5 倍
```

## API 使用示例

```rust
use DistRender::core::{SceneConfig, Transform, CameraConfig};

// 加载场景
let scene = SceneConfig::from_file("scene.toml")?;

// 获取矩阵
let model_matrix = scene.model.transform.to_matrix();
let view_matrix = scene.camera.view_matrix();
let projection_matrix = scene.camera.projection_matrix(16.0 / 9.0);

// MVP 变换
let mvp = projection_matrix * view_matrix * model_matrix;

// 修改场景
let mut scene = SceneConfig::default();
scene.camera.transform.position = [5.0, 5.0, 5.0];
scene.camera.fov = 90.0;

// 保存修改后的场景
scene.save_to_file("new_scene.toml")?;
```

## 关键点总结

1. **坐标系统**：
   - Vulkan：Y 轴向下（已在着色器中翻转）
   - DX12：Y 轴向上
   - 右手坐标系

2. **矩阵顺序**：
   - MVP = Projection × View × Model
   - 在着色器中：`gl_Position = projection * view * model * vec4(position, 1.0)`

3. **内存对齐**：
   - Vulkan：自动处理
   - DX12：常量缓冲区必须 256 字节对齐

4. **性能考虑**：
   - 使用 `CpuBufferPool` 避免每帧创建新的 buffer
   - 只在场景变化时更新 MVP 矩阵

## 下一步

1. 实现完整的 uniform buffer 绑定（参考上述代码）
2. 添加相机控制（键盘/鼠标输入）
3. 支持多个模型实例
4. 添加动画支持（每帧更新变换）

## 相关文件

- 场景配置：[src/core/scene.rs](src/core/scene.rs)
- 配置文件：[scene.toml](scene.toml)
- Vulkan 着色器：[src/renderer/shaders/vertex.glsl](src/renderer/shaders/vertex.glsl)
- DX12 着色器：[src/renderer/shaders/shader.hlsl](src/renderer/shaders/shader.hlsl)
- 顶点结构：[src/renderer/vertex.rs](src/renderer/vertex.rs)
