# DistRender 解耦重构计划

## 概述

本文档提出了一套系统性的解耦方案，旨在改善代码的可维护性、可扩展性和可测试性。

---

## 问题总结

### 🔴 高优先级问题

#### 1. 缺少统一的后端接口抽象
- **当前实现**：使用枚举分发，每个方法都需要手动匹配所有后端
- **影响**：添加新方法需要修改多处，代码重复度高
- **文件**：`src/renderer/mod.rs`

### 🟡 中优先级问题

#### 2. 渲染器直接依赖具体组件类型
- **当前实现**：渲染器直接持有 `Camera` 和 `DirectionalLight`
- **影响**：无法动态管理组件，不支持多光源/多相机
- **文件**：`src/gfx/vulkan/renderer.rs`, `src/gfx/wgpu/renderer.rs`

#### 3. InputSystem 直接操作 Camera
- **当前实现**：`InputSystem::update_camera(&mut Camera)`
- **影响**：输入系统和相机强耦合，无法控制其他对象
- **文件**：`src/core/input.rs`

#### 4. GUI 状态与渲染器紧耦合
- **当前实现**：`GuiStatePacket` 包含场景具体细节
- **影响**：GUI 知道太多渲染细节，协议脆弱
- **文件**：`src/gui/ipc.rs`, `src/renderer/mod.rs`

### 🟠 低优先级问题

#### 5. 缺少资源管理抽象
- **当前实现**：每个后端重复实现资源管理
- **影响**：代码重复，难以统一优化
- **文件**：各后端渲染器

---

## 解决方案

### 方案 1: 引入 RenderBackend Trait ⭐⭐⭐⭐⭐

**目标**：消除枚举分发，使用统一的 trait 接口

#### 实现步骤

**Step 1**：定义 `RenderBackend` trait

```rust
// src/renderer/backend_trait.rs
use crate::core::error::Result;
use crate::core::input::InputSystem;
use crate::gui::ipc::GuiStatePacket;
use winit::window::Window;
use winit::event::WindowEvent;

/// 统一的渲染后端接口
///
/// 所有图形后端（Vulkan, DX12, Metal, wgpu）都必须实现此 trait
pub trait RenderBackend: Send {
    /// 窗口引用
    fn window(&self) -> &Window;
    
    /// 窗口尺寸变化时调用
    fn resize(&mut self);
    
    /// 渲染一帧
    fn draw(&mut self) -> Result<()>;
    
    /// 更新渲染器状态（处理输入、更新相机等）
    fn update(&mut self, input_system: &mut InputSystem, delta_time: f32);
    
    /// 应用 GUI 参数包
    fn apply_gui_packet(&mut self, packet: &GuiStatePacket);
    
    /// 处理 GUI 事件（仅 wgpu 后端需要）
    fn handle_gui_event(&mut self, event: &WindowEvent) -> bool {
        false // 默认实现：不处理
    }
}
```

**Step 2**：修改 `Renderer` 使用 trait object

```rust
// src/renderer/mod.rs
pub struct Renderer {
    backend: Box<dyn RenderBackend>,
}

impl Renderer {
    pub fn new(event_loop: &EventLoop<()>, config: &Config, scene: &SceneConfig) -> Result<Self> {
        use crate::core::config::GraphicsBackend as GfxBackend;
        
        let backend: Box<dyn RenderBackend> = match config.graphics.backend {
            GfxBackend::Vulkan => {
                info!("Initializing Vulkan Backend");
                Box::new(VulkanRenderer::new(event_loop, config, scene)?)
            }
            #[cfg(target_os = "windows")]
            GfxBackend::Dx12 => {
                info!("Initializing DX12 Backend");
                Box::new(Dx12Renderer::new(event_loop, config, scene)?)
            }
            GfxBackend::Wgpu => {
                info!("Initializing wgpu Backend");
                Box::new(WgpuRenderer::new(event_loop, config, scene)?)
            }
            // ... 其他后端
        };

        Ok(Self { backend })
    }

    // 所有方法变成简单的委托
    pub fn draw(&mut self) -> Result<()> {
        self.backend.draw()
    }

    pub fn resize(&mut self) {
        self.backend.resize()
    }

    pub fn window(&self) -> &Window {
        self.backend.window()
    }

    // ... 其他方法类似
}
```

**优势**：
- ✅ 消除了所有 match 分发代码
- ✅ 添加新方法只需修改 trait 和实现
- ✅ 符合开闭原则
- ✅ 代码更简洁

**劣势**：
- ❌ 轻微的虚函数调用开销（通常可忽略）
- ❌ trait object 需要堆分配

---

### 方案 2: 引入场景图和ECS架构 ⭐⭐⭐⭐

**目标**：解耦渲染器和具体组件，支持动态场景管理

#### 实现步骤

**Step 1**：创建 Scene 管理器

```rust
// src/core/scene_manager.rs
use crate::component::{Camera, DirectionalLight, Transform};
use std::collections::HashMap;

pub type EntityId = u64;

/// 场景管理器
///
/// 管理场景中的所有实体和组件
pub struct SceneManager {
    next_entity_id: EntityId,
    
    // 组件存储（简化版 ECS）
    transforms: HashMap<EntityId, Transform>,
    cameras: HashMap<EntityId, Camera>,
    lights: HashMap<EntityId, DirectionalLight>,
    
    // 活跃的主相机
    active_camera: Option<EntityId>,
}

impl SceneManager {
    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            transforms: HashMap::new(),
            cameras: HashMap::new(),
            lights: HashMap::new(),
            active_camera: None,
        }
    }
    
    /// 创建新实体
    pub fn create_entity(&mut self) -> EntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        id
    }
    
    /// 添加相机组件
    pub fn add_camera(&mut self, entity: EntityId, camera: Camera) {
        self.cameras.insert(entity, camera);
    }
    
    /// 获取活跃相机（可变引用）
    pub fn active_camera_mut(&mut self) -> Option<&mut Camera> {
        self.active_camera.and_then(|id| self.cameras.get_mut(&id))
    }
    
    /// 获取所有光源
    pub fn lights(&self) -> impl Iterator<Item = (&EntityId, &DirectionalLight)> {
        self.lights.iter()
    }
    
    // ... 更多方法
}
```

**Step 2**：渲染器持有 SceneManager 引用

```rust
// src/gfx/vulkan/renderer.rs
pub struct Renderer {
    gfx: VulkanBackend,
    // ... 其他字段
    
    // 移除：camera: Camera
    // 移除：directional_light: DirectionalLight
    
    // 不再持有组件，而是通过 SceneManager 访问
}

impl Renderer {
    pub fn update(&mut self, scene: &mut SceneManager, input_system: &mut InputSystem, delta_time: f32) {
        // 获取相机引用
        if let Some(camera) = scene.active_camera_mut() {
            input_system.update_camera(camera, delta_time);
        }
    }
    
    pub fn draw(&mut self, scene: &SceneManager) -> Result<()> {
        // 从场景获取相机和光源
        let camera = scene.active_camera().ok_or(...)?;
        let lights: Vec<_> = scene.lights().collect();
        
        // 使用相机和光源进行渲染
        // ...
    }
}
```

**优势**：
- ✅ 渲染器和组件解耦
- ✅ 支持多相机、多光源
- ✅ 可动态添加/删除组件
- ✅ 为完整 ECS 系统奠定基础

**劣势**：
- ❌ 需要重构较多代码
- ❌ 增加了间接层

---

### 方案 3: 输入系统与相机解耦 ⭐⭐⭐

**目标**：输入系统只产生事件，不直接操作对象

#### 实现步骤

**Step 1**：定义输入指令

```rust
// src/core/input.rs

/// 相机控制指令
#[derive(Debug, Clone, Copy)]
pub enum CameraCommand {
    MoveForward(f32),    // 前进距离
    MoveRight(f32),      // 右移距离
    MoveUp(f32),         // 上移距离
    Rotate(f32, f32),    // (yaw, pitch) 旋转角度
}

/// 输入系统输出
pub struct InputCommands {
    pub camera_commands: Vec<CameraCommand>,
}

impl InputSystem {
    /// 处理输入并生成指令（不再直接操作相机）
    pub fn process(&mut self, delta_time: f32) -> InputCommands {
        let mut commands = InputCommands {
            camera_commands: Vec::new(),
        };
        
        // WASD 移动
        let move_distance = self.move_speed * delta_time;
        if self.is_key_pressed(KeyCode::KeyW) {
            commands.camera_commands.push(CameraCommand::MoveForward(move_distance));
        }
        // ... 其他按键
        
        // 鼠标旋转
        if self.cursor_locked {
            let (dx, dy) = self.mouse_delta;
            if dx != 0.0 || dy != 0.0 {
                commands.camera_commands.push(CameraCommand::Rotate(dx, dy));
            }
        }
        
        self.reset_frame();
        commands
    }
}
```

**Step 2**：在外部应用指令

```rust
// src/main.rs 或 renderer 中
let commands = input_system.process(delta_time);

// 应用到相机
if let Some(camera) = scene.active_camera_mut() {
    for cmd in commands.camera_commands {
        match cmd {
            CameraCommand::MoveForward(dist) => camera.walk(dist),
            CameraCommand::MoveRight(dist) => camera.strafe(dist),
            CameraCommand::MoveUp(dist) => camera.fly(dist),
            CameraCommand::Rotate(yaw, pitch) => camera.rotate(yaw, pitch),
        }
    }
}
```

**优势**：
- ✅ 输入系统和相机完全解耦
- ✅ 可重用输入系统控制其他对象
- ✅ 支持输入录制/回放
- ✅ 更容易测试

**劣势**：
- ❌ 增加了一层间接调用

---

### 方案 4: 引入参数系统（解耦GUI） ⭐⭐⭐⭐

**目标**：GUI 不直接知道场景结构，通过参数系统间接修改

#### 实现步骤

**Step 1**：定义参数系统

```rust
// src/core/parameter.rs
use std::collections::HashMap;

/// 可调整的参数类型
#[derive(Debug, Clone)]
pub enum ParameterValue {
    Float(f32),
    Vec3([f32; 3]),
    Color([f32; 4]),
    Bool(bool),
}

/// 参数系统
pub struct ParameterSystem {
    parameters: HashMap<String, ParameterValue>,
}

impl ParameterSystem {
    pub fn new() -> Self {
        Self {
            parameters: HashMap::new(),
        }
    }
    
    /// 注册参数
    pub fn register(&mut self, name: impl Into<String>, value: ParameterValue) {
        self.parameters.insert(name.into(), value);
    }
    
    /// 设置参数
    pub fn set(&mut self, name: &str, value: ParameterValue) {
        if let Some(param) = self.parameters.get_mut(name) {
            *param = value;
        }
    }
    
    /// 获取参数
    pub fn get(&self, name: &str) -> Option<&ParameterValue> {
        self.parameters.get(name)
    }
    
    /// 获取浮点参数
    pub fn get_float(&self, name: &str) -> Option<f32> {
        match self.get(name) {
            Some(ParameterValue::Float(v)) => Some(*v),
            _ => None,
        }
    }
    
    // ... 其他类型的 getter
}
```

**Step 2**：场景注册参数

```rust
// src/core/scene_manager.rs
impl SceneManager {
    pub fn register_parameters(&self, params: &mut ParameterSystem) {
        // 注册清空颜色
        params.register("scene.clear_color", ParameterValue::Color([0.0, 0.0, 0.2, 1.0]));
        
        // 注册光源参数
        params.register("light.intensity", ParameterValue::Float(1.0));
        params.register("light.direction", ParameterValue::Vec3([0.0, -1.0, 0.0]));
        
        // 注册相机参数
        params.register("camera.fov", ParameterValue::Float(45.0));
        
        // ... 其他参数
    }
    
    pub fn apply_parameters(&mut self, params: &ParameterSystem) {
        // 从参数系统读取并应用到场景
        if let Some(fov) = params.get_float("camera.fov") {
            if let Some(camera) = self.active_camera_mut() {
                camera.set_lens(fov, camera.get_aspect(), camera.get_near(), camera.get_far());
            }
        }
        // ... 应用其他参数
    }
}
```

**Step 3**：GUI 只修改参数

```rust
// src/gui/panels/scene.rs
pub fn render(ui: &mut egui::Ui, params: &mut ParameterSystem) {
    ui.heading("场景参数");
    
    // GUI 只知道参数名称和类型，不知道具体实现
    if let Some(ParameterValue::Color(color)) = params.get("scene.clear_color") {
        let mut color_edit = *color;
        if ui.color_edit_button_rgba_unmultiplied(&mut color_edit).changed() {
            params.set("scene.clear_color", ParameterValue::Color(color_edit));
        }
    }
    
    if let Some(ParameterValue::Float(intensity)) = params.get("light.intensity") {
        let mut value = *intensity;
        if ui.add(egui::Slider::new(&mut value, 0.0..=2.0).text("光照强度")).changed() {
            params.set("light.intensity", ParameterValue::Float(value));
        }
    }
}
```

**优势**：
- ✅ GUI 和场景完全解耦
- ✅ 参数可序列化/反序列化
- ✅ 支持参数动画、插值
- ✅ 可实现参数预设系统

**劣势**：
- ❌ 运行时类型检查开销
- ❌ 字符串查找开销（可用静态 ID 优化）

---

### 方案 5: 统一资源管理器 ⭐⭐

**目标**：抽象资源管理，减少重复代码

#### 概要设计

```rust
// src/renderer/resource_manager.rs
pub trait ResourceManager {
    type Buffer;
    type Texture;
    
    fn create_vertex_buffer(&mut self, data: &[u8]) -> Result<Self::Buffer>;
    fn create_index_buffer(&mut self, data: &[u32]) -> Result<Self::Buffer>;
    fn create_uniform_buffer(&mut self, size: usize) -> Result<Self::Buffer>;
    fn create_texture(&mut self, width: u32, height: u32) -> Result<Self::Texture>;
    
    fn update_buffer(&mut self, buffer: &Self::Buffer, data: &[u8]) -> Result<()>;
}
```

由于不同图形 API 的资源类型差异较大，此方案优先级较低。

---

## 实施建议

### 阶段 1：核心重构（1-2周）
1. ✅ 实施方案 1：引入 `RenderBackend` trait
2. ✅ 实施方案 3：输入系统解耦

### 阶段 2：架构增强（2-3周）
3. ✅ 实施方案 4：参数系统
4. ✅ 实施方案 2：场景管理器（简化版）

### 阶段 3：完善优化（可选）
5. ⚪ 完整 ECS 系统
6. ⚪ 统一资源管理器

---

## 收益评估

| 方案 | 代码减少 | 可维护性 | 可扩展性 | 性能影响 | 实施难度 |
|------|---------|---------|---------|---------|---------|
| 方案1 | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~2% | ⭐⭐ |
| 方案2 | ⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~5% | ⭐⭐⭐⭐ |
| 方案3 | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 0% | ⭐⭐ |
| 方案4 | ⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ~1% | ⭐⭐⭐ |
| 方案5 | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ~3% | ⭐⭐⭐⭐ |

---

## 总结

当前代码整体架构**良好**，但存在一些**过度耦合**的问题。通过实施上述方案，可以显著改善代码的：

- ✅ **可维护性**：减少重复代码，清晰的职责划分
- ✅ **可扩展性**：更容易添加新功能和后端
- ✅ **可测试性**：模块间依赖减少，更容易单元测试
- ✅ **代码质量**：符合 SOLID 原则，更优雅的设计

建议优先实施**方案1**和**方案3**，它们实施难度低但收益高。
