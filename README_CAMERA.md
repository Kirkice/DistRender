# 相机系统使用指南

## 🎉 功能已就绪

相机系统和场景配置已经完全集成到 DistRender 中！

## ✅ 当前状态

### 已完成的功能

1. **场景配置系统** ✅
   - 从 `scene.toml` 加载相机和模型配置
   - 支持位置、旋转、缩放变换
   - 支持 FOV、近/远裁剪面配置

2. **数学支持** ✅
   - Transform → Model Matrix（`to_matrix()`）
   - Camera → View Matrix（`view_matrix()`）
   - Camera → Projection Matrix（`projection_matrix(aspect_ratio)`）

3. **着色器** ✅
   - Vulkan 和 DX12 着色器支持 3D 位置
   - MVP 矩阵 uniform buffer 接口已定义

4. **主程序集成** ✅
   - `main.rs` 已更新，自动加载 `scene.toml`
   - 日志显示场景配置信息

### 运行日志示例

```
INFO DistRender starting...
INFO Loaded scene config from: scene.toml
INFO Scene configuration camera_pos=[0.0, 0.0, 3.0] camera_fov=60.0 model_path=assets/models/sphere.obj
WARN Scene configuration loaded but not yet integrated with renderer
WARN The renderer still uses hardcoded camera/model transforms
```

## 📝 使用场景配置

### 查看当前配置

程序启动时会自动加载 `scene.toml` 并在日志中显示：

```bash
cargo run
```

### 修改相机配置

编辑 `scene.toml`：

```toml
[camera]
[camera.transform]
position = [5.0, 3.0, 5.0]  # 从右上方观察
rotation = [-30.0, 45.0, 0.0]  # 向下 30°，向右旋转 45°
scale = [1.0, 1.0, 1.0]

fov = 75.0        # 更宽的视野
near_clip = 0.1
far_clip = 100.0
```

### 修改模型配置

```toml
[model]
path = "assets/models/sphere.obj"

[model.transform]
position = [0.0, 0.5, 0.0]   # 向上移动
rotation = [0.0, 45.0, 0.0]  # Y 轴旋转 45°
scale = [2.0, 2.0, 2.0]      # 放大 2 倍
```

## 🔧 API 使用示例

场景配置已经可以在代码中使用：

```rust
use DistRender::core::{SceneConfig, Matrix4};

// 加载场景
let scene = SceneConfig::from_file_or_default("scene.toml");

// 获取变换矩阵
let model = scene.model.transform.to_matrix();
let view = scene.camera.view_matrix();
let projection = scene.camera.projection_matrix(16.0 / 9.0);

// 计算 MVP
let mvp = projection * view * model;

// 使用矩阵数据
let model_array: [[f32; 4]; 4] = *model.as_ref();
println!("Model matrix: {:?}", model_array);
```

## 📐 坐标系统说明

### 世界坐标系
- **X 轴**：向右为正
- **Y 轴**：向上为正
- **Z 轴**：向前为正（右手坐标系）

### 欧拉角（Euler Angles）
- **Pitch（俯仰）**：绕 X 轴旋转，正值向下看
- **Yaw（偏航）**：绕 Y 轴旋转，正值向右转
- **Roll（翻滚）**：绕 Z 轴旋转，正值顺时针

所有角度单位为**度数**（不是弧度）。

## 🎨 常用相机设置示例

### 1. 正面视角（默认）
```toml
[camera.transform]
position = [0.0, 0.0, 3.0]
rotation = [0.0, 0.0, 0.0]
```

### 2. 俯视图（从上往下看）
```toml
[camera.transform]
position = [0.0, 5.0, 0.1]
rotation = [-90.0, 0.0, 0.0]
```

### 3. 侧视图
```toml
[camera.transform]
position = [5.0, 0.0, 0.0]
rotation = [0.0, -90.0, 0.0]
```

### 4. 等角投影视角（45°）
```toml
[camera.transform]
position = [3.0, 3.0, 3.0]
rotation = [-35.0, 45.0, 0.0]
```

### 5. 广角镜头
```toml
fov = 90.0  # 更宽的视野
near_clip = 0.1
far_clip = 100.0
```

### 6. 望远镜效果
```toml
fov = 30.0  # 窄视野
near_clip = 1.0
far_clip = 500.0
```

## 🛠️ 下一步：完整渲染器集成

当前场景配置已经被加载，但**尚未传递给渲染器**。要完成集成：

### 步骤 1：修改渲染器接口

**Vulkan 渲染器** ([src/renderer/vulkan.rs](src/renderer/vulkan.rs)):
- 添加 `scene: SceneConfig` 字段
- 在 `new()` 中接受 `scene` 参数
- 创建 uniform buffer pool
- 在 `draw()` 中计算并更新 MVP 矩阵

**DX12 渲染器** ([src/renderer/dx12.rs](src/renderer/dx12.rs)):
- 更新输入布局（vec3 position）
- 添加常量缓冲区
- 在 `draw()` 中更新 MVP 数据

### 步骤 2：修改 main.rs

取消注释：
```rust
// 当前：
let mut renderer = match Renderer::new(&event_loop, &config) {

// 修改为：
let mut renderer = match Renderer::new(&event_loop, &config, &scene) {
```

### 步骤 3：测试

修改 `scene.toml`，运行程序，观察相机和模型变换效果。

## 📚 完整文档

详细的实现步骤和代码示例，请参考：
- **[CAMERA_INTEGRATION.md](CAMERA_INTEGRATION.md)** - 渲染器集成详细指南
- **[src/core/scene.rs](src/core/scene.rs)** - 场景配置 API 文档
- **[scene.toml](scene.toml)** - 配置文件模板

## 🎯 测试清单

- [x] 场景配置文件可以被加载
- [x] 日志显示正确的配置信息
- [x] 可以修改 scene.toml 并重新运行
- [ ] MVP 矩阵传递给着色器
- [ ] 相机位置影响渲染结果
- [ ] 模型变换影响渲染结果
- [ ] FOV 调整影响透视效果

## 💡 提示

1. **调试相机位置**：修改 `camera.transform.position`，观察日志中的变化
2. **测试不同 FOV**：尝试 30°（望远）到 120°（超广角）
3. **旋转模型**：使用 `model.transform.rotation` 查看不同角度
4. **多个模型**：未来可以扩展支持模型数组

## 🐛 故障排除

### 问题：场景配置未加载
**解决**：确保 `scene.toml` 在项目根目录

### 问题：日志中看不到场景信息
**解决**：检查日志级别设置（应该至少为 INFO）

### 问题：修改配置后没有效果
**解决**：
1. 目前渲染器还没有使用场景配置
2. 需要按照 CAMERA_INTEGRATION.md 完成集成

---

**当前版本**：场景配置系统已集成 ✅
**下一步**：渲染器 MVP uniform buffer 集成 ⏳
