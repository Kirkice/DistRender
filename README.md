# DistRender - 分布式渲染引擎

一个支持多图形 API 的现代化渲染引擎，目前支持 Vulkan 和 DirectX 12。

## ✨ 特性

- 🎨 **多后端支持**：支持 Vulkan（跨平台）和 DirectX 12（Windows）
- 🔧 **统一接口**：提供一致的渲染 API，无缝切换图形后端
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
└── Cargo.toml
```

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

### 架构优化
- ✅ 创建 `GraphicsBackend` trait 统一后端接口
- ✅ 重命名 `GfxDevice` 为 `VulkanBackend`，保持命名一致性
- ✅ 改进 `gfx` 和 `renderer` 模块结构

### 文档优化
- ✅ 为所有模块添加完整的中文文档注释
- ✅ 添加详细的使用示例和架构说明
- ✅ 为关键函数添加参数、返回值和异常说明

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
