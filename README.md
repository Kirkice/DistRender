# Dist Render

基于 [Embark Studios / kajiya](https://github.com/EmbarkStudios/kajiya) 的实时全局光照渲染器，使用 **Rust + Vulkan** 构建，支持硬件光线追踪。

---

## 功能特性

- **实时全局光照 (GI)** — 多 bounce 漫反射 (RTDGI) 和镜面反射 (RTR)
- **硬件光线追踪** — Vulkan Ray Tracing 管线驱动的阴影、反射及间接光照
- **物理天空模型** — 大气散射 + HDR 环境贴图 (IBL)
- **后处理管线** — TAA、运动模糊、自动曝光、景深、SSGI 等
- **现代编辑器 UI** — egui 驱动的 Hierarchy / Inspector / Viewport 三栏布局，Unity/Unreal 风格暗色主题
- **glTF 场景导入** — 支持 PBR 材质、双面材质、自发光材质
- **场景图 & 组件系统** — Camera / Sun / LocalLights / MeshRenderer 组件，支持父子层级
- **可选 DLSS 支持** — 通过 `dlss` feature flag 启用 NVIDIA DLSS 超分

## 系统要求

| 项目 | 要求 |
|------|------|
| 操作系统 | Windows 10/11 (64-bit) |
| GPU | 支持 Vulkan 1.2 + Ray Tracing 的显卡 (NVIDIA RTX 20 系列及以上) |
| Rust | stable 工具链 (推荐 1.75+) |
| 磁盘 | 约 2 GB（含资产和编译缓存） |

## 快速开始

### 1. 克隆仓库

```bash
git clone <repo-url> DistRender
cd DistRender
```

### 2. 编译并运行

```bash
# Debug 模式运行（默认加载 pica 场景）
cargo run --bin renderer

# Release 模式运行（推荐，性能更好）
cargo run --bin renderer --release
```

### 3. 指定场景

```bash
cargo run --bin renderer -- --scene assets/scenes/pica.ron
```

### 4. 直接加载 glTF 模型

```bash
cargo run --bin renderer -- --mesh assets/meshes/cornell_box/scene.gltf --mesh-scale 1.0
```

## 命令行参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `--width` | `1920` | 窗口宽度 |
| `--height` | `1080` | 窗口高度 |
| `--scene <path>` | — | `.ron` 场景描述文件路径 |
| `--mesh <path>` | — | 直接加载单个 glTF/mesh 文件 |
| `--mesh-scale <f32>` | `1.0` | 模型缩放系数 |
| `--temporal-upsampling <f32>` | `1.0` | 时序上采样比例 |
| `--no-vsync` | `false` | 关闭垂直同步 |
| `--fullscreen` | `false` | 全屏模式 |
| `--graphics-debugging` | `false` | 启用 Vulkan 验证层 |
| `--physical-device-index <n>` | — | 指定 GPU 设备索引 |
| `--keymap <path>` | — | 自定义键位配置文件 |
| `--no-window-decorations` | `false` | 无边框窗口 |

## 内置场景

`assets/scenes/` 目录下提供以下预配置场景：

| 场景文件 | 说明 |
|----------|------|
| `pica.ron` | Pica Pica 微缩场景 (默认) |
| `conference.ron` | 会议室场景 |
| `cornell_box.ron` | Cornell Box 经典测试场景 |
| `battle.ron` | 战斗场景 |
| `mini_battle.ron` | 缩小版战斗场景 |
| `car.ron` | 汽车场景 |
| `gas_stations.ron` | 加油站场景 |
| `roughness-scale.ron` | 粗糙度梯度测试 |
| `viziers.ron` | 观景台场景 |

## 项目结构

```
DistRender/
├── Cargo.toml                  # workspace 根配置
├── assets/
│   ├── fonts/                  # UI 字体
│   ├── images/                 # 蓝噪声等纹理
│   ├── meshes/                 # glTF 模型资产
│   ├── rust-shaders-compiled/  # 预编译 Rust-GPU 着色器
│   ├── scenes/                 # .ron 场景描述文件
│   └── shaders/                # HLSL 着色器源码
│       ├── rt/                 # 光线追踪着色器
│       ├── rtdgi/              # 漫反射 GI
│       ├── rtr/                # 镜面反射
│       ├── ssgi/               # 屏幕空间 GI
│       ├── taa/                # 时序抗锯齿
│       ├── shadow_denoise/     # 阴影降噪
│       ├── sky/                # 天空模型
│       ├── lighting/           # 光照计算
│       ├── ibl/                # 环境光照
│       ├── ircache/            # 辐照度缓存
│       ├── wrc/                # 世界辐射缓存
│       ├── motion_blur/        # 运动模糊
│       ├── dof/                # 景深
│       ├── post/               # 后处理
│       └── ...
├── cache/                      # 烘焙后的资产缓存
├── crates/
│   ├── bin/
│   │   ├── renderer/           # 主渲染器 / 编辑器应用
│   │   └── bake/               # 离线资产烘焙工具
│   └── lib/
│       ├── dist-render/            # 渲染核心 (render passes 编排)
│       ├── dist-render-backend/    # Vulkan/ash 后端
│       ├── dist-render-rg/         # 渲染图 (Render Graph)
│       ├── dist-render-asset/      # 资产类型定义
│       ├── dist-render-asset-pipe/ # 资产处理管线
│       ├── dist-render-simple/     # 上层简化 API
│       ├── dist-render-egui/       # egui 渲染后端
│       ├── rust-shaders/           # Rust-GPU 着色器
│       └── rust-shaders-shared/    # CPU/GPU 共享类型
└── docs/                       # 文档
```

## 资产烘焙

大型 glTF 模型首次加载后会自动缓存到 `cache/` 目录。也可手动预烘焙：

```bash
cargo run --bin bake -- --scene assets/scenes/pica.ron -o pica_baked
```

> **注意**：修改材质标志（如双面材质）后需要删除 `cache/*.mesh` 文件以触发重新烘焙。

## 编辑器操作

| 操作 | 按键 |
|------|------|
| 显示/隐藏 UI | 由 keymap 配置决定 |
| 摄像机移动 | `W` `A` `S` `D` + 鼠标右键 |
| 摄像机升降 | `Q` / `E` |
| 拖拽 IBL 环境贴图 | 将 `.hdr` / `.exr` 文件拖入窗口 |
| 选择物体 | 在 Viewport 中左键点击 |
| 移动选中物体 | 拖拽 Gizmo 轴 |

Inspector 面板可编辑：
- **场景级**：曝光、环境光、光照、渲染设置
- **物体级**：Transform（位置/旋转/缩放）、组件属性（Camera / Sun / LocalLights / MeshRenderer）

## 可选功能

通过 Cargo feature flags 启用：

```bash
# 启用 NVIDIA DLSS
cargo run --bin renderer --features dlss

# 启用 puffin 性能分析服务器
cargo run --bin renderer --features puffin-server
```

## 许可证

本项目基于 [Embark Studios / kajiya](https://github.com/EmbarkStudios/kajiya) 开发，采用双许可证：

- [Apache License 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

Copyright (c) 2019 Embark Studios
