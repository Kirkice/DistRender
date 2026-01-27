# DistRender - 渲染引擎

一个支持多图形 API 的现代化渲染引擎，目前支持 Vulkan 和 DirectX 12。

## ✨ 特性

- 🎨 **多后端支持**：支持 Vulkan（跨平台）和 DirectX 12（Windows）
- 🔧 **统一接口**：提供一致的渲染 API，无缝切换图形后端
- ⚡ **事件系统**：类型安全、零成本抽象的事件处理框架
- 📚 **完整文档**：所有模块都有详细的中文文档注释
- 🚀 **高性能**：使用现代图形 API，充分发挥 GPU 性能
- 🛠️ **模块化设计**：清晰的模块划分，易于维护和扩展

## 🚀 快速开始

### 使用 Vulkan（默认）

```bash
cargo run
```

### 使用 DirectX 12

```bash
cargo run -- --dx12
```

### Release 模式

```bash
cargo run --release
```

## 📁 项目结构

```
DistRender/
├── src/
│   ├── main.rs              # 应用程序入口
│   ├── core/                # 核心功能模块
│   │   ├── math/            # 数学库
│   │   ├── log.rs           # 日志系统
│   │   ├── config.rs        # 配置管理
│   │   ├── error.rs         # 错误处理
│   │   └── event.rs         # 事件系统 ⭐
│   ├── gfx/                 # 图形后端模块
│   │   ├── mod.rs           # 模块导出
│   │   ├── backend.rs       # GraphicsBackend trait 定义
│   │   ├── vulkan.rs        # Vulkan 后端实现
│   │   └── dx12.rs          # DirectX 12 后端实现
│   └── renderer/            # 渲染器模块
│       ├── mod.rs           # 统一渲染接口
│       ├── vulkan.rs        # Vulkan 渲染器
│       ├── dx12.rs          # DX12 渲染器
│       ├── vertex.rs        # 顶点数据定义
│       └── shaders.rs       # 着色器程序
├── examples/
│   └── event_system_demo.rs # 事件系统演示
└── Cargo.toml
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

### 快速开始

```rust
use DistRender::core::event::*;

fn main() {
    // 1. 创建事件
    let mut event = WindowResizeEvent::new(1920, 1080);

    // 2. 创建分发器
    let mut dispatcher = EventDispatcher::new(&mut event);

    // 3. 处理事件
    dispatcher.dispatch(EventType::WindowResize, |e| {
        println!("窗口调整为: {}", e.detail());
        true // 标记为已处理
    });

    // 4. 检查处理状态
    println!("事件已处理: {}", dispatcher.is_handled());
}
```

### 运行演示

```bash
cargo run --example event_system_demo
```

### 架构对比

| 特性 | DistEngine (C++) | DistRender (Rust) |
|------|------------------|-------------------|
| 事件基类 | `Event` 抽象类 | `Event` trait |
| 类型检查 | 运行时 `dynamic_cast` | 编译时类型匹配 |
| 分发机制 | 模板 + unsafe cast | 模式匹配 + 闭包 |
| 性能 | 虚函数调用开销 | 零成本抽象 |

### 使用示例：WASD 移动控制

```rust
use DistRender::core::event::*;

struct CameraController {
    position: (f32, f32, f32),
    speed: f32,
}

impl CameraController {
    fn handle_key(&mut self, event: &KeyboardEvent, dt: f32) {
        if !event.pressed { return; }

        let distance = self.speed * dt;
        match event.key_code {
            KeyCode::W => self.position.2 += distance,
            KeyCode::S => self.position.2 -= distance,
            KeyCode::A => self.position.0 -= distance,
            KeyCode::D => self.position.0 += distance,
            _ => {}
        }
    }
}
```

更多详细文档和教程，请运行 `cargo doc --open` 查看完整 API 文档。

## 🏗️ 架构设计

### 分层架构

```
┌─────────────────────────────────┐
│         Application            │  应用层（main.rs）
├─────────────────────────────────┤
│     Renderer（统一接口）       │  渲染层
├────────────┬────────────────────┤
│  Vulkan    │     DirectX 12     │  后端层
│  Renderer  │     Renderer       │
├────────────┼────────────────────┤
│  Vulkan    │     DirectX 12     │  图形 API 层
│  Backend   │     Backend        │
└────────────┴────────────────────┘
```

### 核心设计原则

1. **抽象化**：通过 `GraphicsBackend` trait 定义统一接口
2. **零成本抽象**：使用枚举而非 trait object，避免动态分发
3. **模块化**：清晰的职责划分，高内聚低耦合
4. **文档化**：完整的 Rustdoc 注释，易于理解和维护

## 📖 API 文档

生成并查看完整的 API 文档：

```bash
cargo doc --open
```

## 🔧 技术栈

- **Rust**：系统编程语言，提供内存安全和高性能
- **Vulkan**：通过 vulkano 库使用
- **DirectX 12**：通过 windows-rs 库使用
- **Winit**：跨平台窗口管理

## 📝 最近优化

### 核心功能
- ✅ **事件系统**：实现类型安全、零成本抽象的事件处理框架
  - 支持窗口、鼠标、键盘、系统等 8 种事件类型
  - 提供事件处理链和状态追踪
  - 完整的单元测试和使用示例
  - 详细的中文文档和注释

### 架构优化
- ✅ 创建 `GraphicsBackend` trait 统一后端接口
- ✅ 重命名 `GfxDevice` 为 `VulkanBackend`，保持命名一致性
- ✅ 改进 `gfx` 和 `renderer` 模块结构
- ✅ 实现帧资源管理和同步机制

### 文档优化
- ✅ 为所有模块添加完整的中文文档注释
- ✅ 添加详细的使用示例和架构说明
- ✅ 为关键函数添加参数、返回值和异常说明
- ✅ 提供事件系统演示程序

### 代码质量
- ✅ 清理未使用的导入
- ✅ 修正项目名称（RustroverProjects → DistRender）
- ✅ 改进错误处理和调试输出
- ✅ 添加内联注释说明关键逻辑

## 🎯 未来计划

- [ ] 支持更多图形后端（Metal、OpenGL）
- [ ] 实现更复杂的渲染场景
- [ ] 添加纹理和材质系统
- [ ] 实现相机控制
- [ ] 添加性能分析工具

## 📄 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！
