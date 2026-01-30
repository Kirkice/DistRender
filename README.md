# DistRender

一个基于 Rust 的现代化跨平台渲染引擎，支持多图形后端（Vulkan、DirectX 12、Metal、wgpu）。

## 📖 项目简介

DistRender 是一个模块化的实时渲染引擎，旨在提供统一的图形 API 抽象，让开发者无需关心底层图形 API 的差异。引擎采用 trait-based 设计模式，通过动态分发实现后端切换，同时保持代码的清晰性和可维护性。

项目的核心设计理念：
- **跨平台优先**：一套代码，多个平台
- **后端无关**：统一接口隐藏图形 API 差异
- **模块化架构**：组件松耦合，易于扩展和测试
- **零成本抽象**：Rust 强类型保证 + 编译期优化

## ✨ 特性

- 🎨 **多后端支持**：支持 Vulkan（跨平台）、DirectX 12（Windows）、Metal（macOS）与 wgpu（跨平台抽象后端）
- 🔧 **统一接口**：统一的 `renderer::Renderer` 接口，在运行时选择后端
- 🎛️ **GUI 系统**：
  - wgpu 后端：内置 egui 面板
  - Vulkan/DX12/Metal 后端：外部 GUI 进程（`dist_render_gui`）+ 共享内存同步参数
- 🖱️ **输入系统**：基于 winit 的键鼠输入，支持 WASD 移动与右键拖拽视角
- ⚡ **事件系统**：类型安全、零成本抽象的事件处理框架
- 🛠️ **模块化设计**：清晰的模块划分，易于维护和扩展

## 🚀 快速开始

### 运行主程序（默认）

由于工程包含多个二进制程序（主渲染器 + 外部 GUI），已在 `Cargo.toml` 配置 `default-run = "dist_render"`，因此以下命令会默认运行主程序：

```bash
cargo run
```

### 选择图形后端

- Vulkan：

```bash
cargo run -- --vulkan
```

- DirectX 12：

```bash
cargo run -- --dx12
```

- Metal（macOS）：

```bash
cargo run -- --metal
```

- wgpu：

```bash
cargo run -- --wgpu
```

### 外部 GUI（仅 Vulkan/DX12/Metal 默认启用）

当使用 Vulkan / DX12 / Metal 后端时，主程序会自动启动外部 GUI 程序 `dist_render_gui`，并通过共享内存把 GUI 参数同步到渲染后端。

你也可以通过命令行控制：

- 强制启用外部 GUI：

```bash
cargo run -- --external-gui
```

- 禁用外部 GUI：

```bash
cargo run -- --no-external-gui
```

### 单独运行外部 GUI 程序

```bash
cargo run --bin dist_render_gui
```

说明：主程序启动外部 GUI 时，会按以下顺序查找可执行文件：

- **优先（B）**：主程序可执行文件同目录下的 `dist_render_gui(.exe)`
- **兜底（A）**：`target/debug/dist_render_gui(.exe)`

### Release 模式

```bash
cargo run --release
```

## 📁 项目结构

```
DistRender/
├── src/
│   ├── main.rs                    # 主渲染程序入口
│   ├── lib.rs                     # 库入口
│   ├── bin/
│   │   └── dist_render_gui.rs     # 外部 GUI 程序
│   │
│   ├── math/                      # 数学库（顶层模块）
│   │   ├── mod.rs                 # 向量、矩阵、四元数、颜色
│   │   └── geometry.rs            # 几何处理（法线、切线计算）
│   │
│   ├── core/                      # 核心系统
│   │   ├── config.rs              # 配置管理
│   │   ├── error.rs               # 错误类型定义
│   │   ├── event.rs               # 事件系统
│   │   ├── input.rs               # 输入处理
│   │   ├── log.rs                 # 日志系统
│   │   ├── runtime.rs             # 运行时管理
│   │   └── scene.rs               # 场景管理
│   │
│   ├── component/                 # 组件系统
│   │   ├── component.rs           # 组件 trait
│   │   ├── camera.rs              # 相机组件
│   │   ├── light.rs               # 光照组件
│   │   ├── transform.rs           # 变换组件
│   │   └── game_object.rs         # 游戏对象容器
│   │
│   ├── geometry/                  # 几何数据
│   │   ├── mesh.rs                # 网格数据结构
│   │   ├── vertex.rs              # 顶点格式
│   │   └── loaders/               # 模型加载器
│   │       ├── obj_loader.rs      # Wavefront OBJ
│   │       └── fbx_loader.rs      # Autodesk FBX
│   │
│   ├── renderer/                  # 渲染器层
│   │   ├── mod.rs                 # 统一 Renderer 接口
│   │   ├── backend_trait.rs       # RenderBackend trait
│   │   ├── resources/             # 渲染资源
│   │   │   ├── vertex.rs          # 顶点格式定义
│   │   │   ├── resource.rs        # 资源池管理
│   │   │   └── descriptor.rs      # 描述符管理
│   │   └── commands/              # 渲染命令
│   │       ├── command.rs         # 命令缓冲
│   │       └── sync.rs            # 同步原语（围栏）
│   │
│   ├── 

```
┌───────────────────────────────────────────────────────┐
│                   Application Layer                  │
│              (main.rs + Runtime System)              │
├───────────────────────────────────────────────────────┤
│                   Renderer Interface                 │
│           (统一的 Renderer + RenderBackend)           │
├──────────┬──────────┬──────────┬────────────────────┤
│  Vulkan  │   DX12   │  Metal   │       wgpu        │
│ Context  │ Context  │ Context  │      Context      │
│    +     │    +     │    +     │         +         │
│ Renderer │ Renderer │ Renderer │     Renderer      │
└──────────┴──────────┴──────────┴────────────────────┘
         ↑          ↑          ↑            ↑
         └──────────┴──────────┴────────────┘
                 RenderBackend Trait
```

### 核心设计模式

#### 1. Trait-based Backend Abstraction（trait 后端抽象）

所有图形后端实现统一的 `RenderBackend` trait：

```rust
pub trait RenderBackend: Send {
    fn window(&self) -> &Window;
    fn resize(&mut self);
    fn draw(&mut self) -> Result<()>;
    fn update(&mut self, input_system: &mut InputSystem, delta_time: f32);
    fn apply_gui_packet(&mut self, packet: &GuiStatePacket);
    // ... 其他方法
}
```

**优势**：
- ✅ 消除了枚举分发的代码重复（从 32 个 match 分支减少为 1 个 trait 调用）
- ✅ 新增后端只需实现 trait，无需修改上层代码
- ✅ 编译期类型检查保证接口一致性

#### 2. Component System（组件系统）

基于 trait 的组件系统，支持动态类型组合：

```rust
pub trait Component: Any {
    fn name(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct GameObject {
    components: HashMap<TypeId, Vec<Box<dyn Component>>>,
    // ...
}
```

**特性**：
- ✅ 类型安全的组件获取（通过 `TypeId`）
- ✅ 支持同一类型的多个组件实例
- ✅ 零成本的类型转换（`downcast_ref`）

#### 3. Event System（事件系统）

类型安全的事件处理框架：

```rust
pub trait Event: Any {
    fn name(&self) -> &'static str;
    fn category(&self) -> EventCategory;
}

pub trait EventHandler<E: Event> {
    fn on_event(&mut self, event: &E) -> bool;
}
```

**特性**：
- ✅ 编译期类型检查
- ✅ 支持事件处理链
- ✅ 零成本抽象（内联优化）

#### 4. Resource Management（资源管理）

分层的资源管理策略：

- **FrameResourcePool**：帧内资源复用（三重缓冲）
- **DescriptorManager**：描述符分配和管理
- **FenceManager**：GPU 同步管理

### 模块依赖关系

```
main.rs
  ├─→ core::Runtime
  │     ├─→ core::Config
  │     ├─→ core::InputSystem
  │     └─→ core::EventDispatcher
  │
  ├─→ renderer::Renderer
  │     ├─→ renderer::RenderBackend (trait)
  │     │     ├─→ gfx::vulkan::Renderer
  │     │     ├─→ gfx::dx12::Renderer
  │     │     ├─→ gfx::metal::Renderer
  │     │     └─→ gfx::wgpu::Renderer
  │     │
  │     ├─→ renderer::resources::*
  │     └─→ renderer::commands::*
  │
  ├─→ gui::GuiManager
  │     ├─→ gui::ipc (外部 GUI 通信)
  │     └─→ egui (wgpu 内置 GUI)
  │
  └─→ component::Camera
        └─→ component::Transform
```

### 关键技术实现

#### Shader 管理

- **编译期编译**：通过 `build.rs` 在构建时编译所有着色器
- **后端特定**：每个后端有独立的着色器目录（GLSL/HLSL/MSL/WGSL）
- **嵌入二进制**：编译后的着色器嵌入可执行文件

#### 跨平台窗口

- **winit**：统一的窗口和事件循环抽象
- **raw-window-handle**：平台无关的窗口句柄传递

#### 进程间通信（IPC）

外部 GUI 使用共享内存与主进程通信：

```rust
pub struct GuiStatePacket {
    pub camera_params: CameraParams,
    pub light_params: LightParams,
    pub render_params: RenderParams,
    // ...
}
```

- **平台抽象**：使用 `shared_memory` crate
- **无锁设计**：原子操作保证数据一致性
- **低延迟**：< 1ms 的参数同步延迟 │   │   ├── descriptor.rs      # 描述符堆管理
│   │   │   └── shaders/           # DX12 着色器（HLSL）
│   │   ├── metal/                 # Metal 实现
│   │   │   ├── context.rs         # 设备上下文
│   │   │   ├── renderer.rs        # 渲染器
│   │   │   └── shaders/           # Metal 着色器（MSL）
│   │   └── wgpu/                  # wgpu 实现
│   │       ├── context.rs         # 设备上下文
│   │       ├── renderer.rs        # 渲染器
│   │       └── shaders/           # wgpu 着色器（WGSL）
### 核心依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| **vulkano** | 0.34 | Vulkan 高级封装 |
| **ash** | 0.38 | Vulkan 低级绑定 |
| **windows** | 0.62.2 | DirectX 12 绑定 |
| **metal** | 0.27.0 | Metal API 绑定 (macOS) |
| **wgpu** | 0.19 | 跨平台图形抽象 |
| **winit** | 0.29 | 窗口和事件管理 |
| **nalgebra** | 0.33 | 线性代数库 |
| **egui** | 0.26 | 即时模式 GUI |
| **tracing** | 0.1 | 结构化日志 |
| **anyhow** | 1.0 | 错误处理 |
| **serde** | 1.0 | 序列化/反序列化 |
| **shared_memory** | 0.12 | 跨进程共享内存 |

### 构建工具

- **shaderc**：GLSL/HLSL 着色器编译
- **spirv-cross**：着色器反射和转换（可选）

## 🔨 构建方法

### 基础构建

```bash
# 克隆仓库
git clone https://github.com/yourusername/DistRender.git
cd DistRender

# Debug 模式构建
cargo build

# Release 模式构建（推荐用于性能测试）
cargo build --release
```

### 构建特定组件

```bash
# 仅构建主程序
cargo build --bin dist_render

# 仅构建外部 GUI
cargo build --bin dist_render_gui

# 构建并运行示例
cargo run --example event_system_demo
cargo run --example load_obj
```

### 编译优化选项

编辑 `Cargo.toml` 可调整优化级别：

```toml
[profile.release]
opt-level = 3           # 最大优化
lto = true              # 链接时优化
codegen-units = 1       # 单编译单元（更好的优化）
strip = true            # 去除调试符号
```

### 构建故障排查

#### 着色器编译失败

```bash
# 确保安装了 CMake
cmake --version

# 清理并重新构建
cargo clean
cargo build
```

#### Vulkan 驱动问题（Linux）

```bash
# 安装 Vulkan 开发包
sudo apt-get install vulkan-tools libvulkan-dev

# 验证 Vulkan 可用
vulkaninfo
```

#### DirectX 12 问题（Windows）

确保 Windows 10/11 版本足够新：

```powershell
# 检查 DirectX 版本
dxdiag
```

## 🚧 未来计划 (TODO)

### 短期目标（1-3 个月）

- [ ] **PBR 材质系统**
  - [ ] 实现基于物理的 BRDF
  - [ ] 支持金属度/粗糙度工作流
  - [ ] HDR 环境贴图

- [ ] **延迟渲染管线**
  - [ ] G-Buffer 实现
  - [ ] 多光源支持（点光源、聚光灯）
  - [ ] SSAO（屏幕空间环境光遮蔽）

- [ ] **资源管理优化**
  - [ ] 纹理加载和缓存
  - [ ] 统一的资源池
  - [ ] 异步资源加载

- [ ] **相机系统增强**
  - [ ] 相机动画路径
  - [ ] 多相机切换
  - [ ] 相机抖动效果

### 中期目标（3-6 个月）

- [ ] **阴影系统**
  - [ ] 级联阴影贴图（CSM）
  - [ ] 软阴影（PCF/PCSS）
  - [ ] 点光源阴影（Cubemap）

- [ ] **后处理管线**
  - [ ] Bloom（泛光）
  - [ ] Tone Mapping（色调映射）
  - [ ] Color Grading（颜色分级）
  - [ ] 抗锯齿（TAA/FXAA）

- [ ] **场景管理**
  - [ ] 场景序列化/反序列化
  - [ ] 场景节点层级
  - [ ] 场景导入导出（glTF 2.0）

- [ ] **性能优化**
  - [ ] 遮挡剔除
  - [ ] LOD（细节层次）系统
  - [ ] GPU Instancing
  - [ ] 多线程渲染命令生成

### 长期目标（6-12 个月）

- [ ] **高级渲染技术**
  - [ ] 光线追踪（DXR/Vulkan Ray Tracing）
  - [ ] 全局光照（GI）
  - [ ] 体积雾/云
  - [ ] 粒子系统

- [ ] **物理系统集成**
  - [ ] 刚体物理（rapier/PhysX）
  - [ ] 碰撞检测
  - [ ] 物理材质

- [ ] **动画系统**
  - [ ] 骨骼动画
  - [ ] 蒙皮网格
  - [ ] 动画混合树

- [ ] **编辑器开发**
  - [ ] 可视化场景编辑器
  - [ ] 材质编辑器
  - [ ] 实时预览

- [ ] **网络渲染**
  - [ ] 多机协同渲染
  - [ ] 渲染农场支持
  - [ ] 远程调试工具

### 平台扩展

- [ ] **移动平台支持**
  - [ ] iOS (Metal)
  - [ ] Android (Vulkan)

- [ ] **Web 平台**
  - [ ] WebGPU 支持
  - [ ] WASM 编译

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

在提交代码前，请确保：
1. 代码符合 Rust 风格指南（`cargo fmt`）
2. 通过所有测试（`cargo test`）
3. 通过 Clippy 检查（`cargo clippy`）
4. 添加必要的注释和文档

## 📚 学习资源

- [Vulkan Tutorial](https://vulkan-tutorial.com/)
- [Learn wgpu](https://sotrh.github.io/learn-wgpu/)
- [Real-Time Rendering](https://www.realtimerendering.com/)
- [GPU Gems](https://developer.nvidia.com/gpugems/gpugems/contributors)

## 📄 许可证

MIT License

Copyright (c) 2024-2026 DistRender Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

**Made with ❤️ using Rust** └── scene.rs           # 场景控制面板
│
├── assets/
│   └── models/                    # 3D 模型资源
│
├── examples/                      # 示例程序
│   ├── event_system_demo.rs       # 事件系统演示
│   └── load_obj.rs                # OBJ 模型加载演示
│
├── build.rs                       # 构建脚本（Shader 编译）
├── Cargo.toml                     # 项目配置
├── config.toml                    # 运行时配置
└── scene.toml                     # 场景配置
```

## ⚡ 事件系统

DistRender 提供了一个类型安全、高性能的事件处理框架，参考了 DistEngine (C++) 的设计理念。

### 特性

- ✅ **类型安全** - 编译时类型检查，避免运行时错误
- ✅ **零成本抽象** - 无运行时开销，性能媲美手写代码
- ✅ **事件处理链** - 支持多个处理器链式处理同一事件
- ✅ **易于扩展** - 轻松添加新的事件类型

### 支持的事件类型

| 类别 | 事件类型 | 说明 |
|------|---------|------|
| **窗口** | `WindowResizeEvent` | 窗口大小调整 |
| | `WindowCloseEvent` | 窗口关闭 |
| **鼠标** | `MouseButtonEvent` | 鼠标按钮按下/释放 |
| | `MouseMoveEvent` | 鼠标移动 |
| | `MouseScrollEvent` | 鼠标滚轮 |
| **键盘** | `KeyboardEvent` | 键盘按键按下/释放 |
| **系统** | `TickEvent` | 每帧时钟事件 |
| | `DrawEvent` | 绘制事件 |

### 运行演示

```bash
cargo run --example event_system_demo
```

## 🏗️ 架构设计

### 分层架构（概念）

```
┌─────────────────────────────────────────────────┐
│              Application                       │  应用层（main.rs）
├─────────────────────────────────────────────────┤
│          Renderer（统一接口）                   │  渲染层
├──────────┬──────────┬──────────┬───────────────┤
│  Vulkan  │   DX12   │  Metal   │     wgpu      │  后端层
│ Renderer │ Renderer │ Renderer │   Renderer    │
└──────────┴──────────┴──────────┴───────────────┘
```

## � 依赖要求

### 通用依赖

- **Rust**：推荐使用最新稳定版（1.70+）
- **CMake**：用于编译 shaderc（Shader 编译库）

### macOS - Metal 后端

Metal 后端是 macOS 原生图形 API，需要以下环境：

- **macOS 10.13+**（High Sierra 或更高版本）
- **Xcode Command Line Tools**：
  ```bash
  xcode-select --install
  ```

- **依赖 crate**：
  - `metal = "0.27.0"` - Metal API 绑定
  - `objc = "0.2.7"` - Objective-C 运行时
  - `cocoa = "0.25.0"` - macOS AppKit 集成

### 其他平台

- **Windows**：DirectX 12 需要 Windows 10+
- **Linux**：Vulkan 需要安装对应驱动

### Shader 编译

所有后端都需要 **shaderc** 用于在构建时编译 Shader：

```bash
# macOS
brew install cmake

# Ubuntu/Debian
sudo apt-get install cmake

# Windows
# 通过 Visual Studio Installer 安装 CMake
```

## ⚠️ 注意事项

### Metal 后端（macOS）

1. **坐标系统**：Metal 使用 Y-up 坐标系（与 OpenGL 一致），深度范围 [0, 1]
2. **Shader 语言**：使用 Metal Shading Language (MSL)，Shader 文件位于 `src/gfx/metal/shaders/shader.metal`
3. **性能优化**：
   - 已启用三重缓冲（`maximum_drawable_count = 3`）以减少帧延迟
   - Depth correction 矩阵已预计算并缓存
4. **GUI 支持**：Metal 后端默认启用外部 GUI，可通过 `--no-external-gui` 禁用

### 跨后端开发

由于不同图形 API 的坐标系统差异，在实现新功能时需注意：

| 后端 | NDC Y 轴 | 深度范围 | 备注 |
|------|---------|---------|------|
| OpenGL/Metal | Y-up | [0, 1] (Metal) / [-1, 1] (GL) | Metal 需深度校正 |
| Vulkan | Y-down | [0, 1] | 需 Y 轴翻转 |
| DirectX 12 | Y-up | [0, 1] | 与 Metal 类似 |
| wgpu | 后端依赖 | 后端依赖 | 自动处理差异 |

## 🔧 技术栈

- **Rust**：系统编程语言
- **Winit**：跨平台窗口管理
- **Vulkan**：通过 `vulkano`/`ash`
- **DirectX 12**：通过 `windows-rs`
- **Metal**：通过 `metal-rs` (macOS 原生)
- **wgpu**：跨平台图形抽象
- **egui**：GUI 框架（wgpu 内置渲染；Vulkan/DX12/Metal 通过外部 GUI + IPC 同步）

## 📄 许可证

MIT License
