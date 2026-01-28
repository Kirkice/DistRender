# Component 组件系统

参考 DistEngine 的 Component 架构在 Rust 中实现的游戏对象组件系统。

## 概述

组件系统提供了以下核心类型：

- **Component** - 组件基础 trait
- **Transform** - 变换组件（位置、旋转、缩放）
- **Camera** - 相机组件
- **GameObject** - 游戏对象容器

## 架构对比

### DistEngine (C++)

```cpp
Component (基类)
├── Transform
├── Camera : Transform
└── GameObject : Component
    └── vector<Component*> components
```

### DistRender (Rust)

```rust
Component (trait)
├── Transform (struct, impl Component)
├── Camera (struct, impl Component)
│   └── transform: Transform (组合)
└── GameObject (struct, impl Component)
    └── Vec<ComponentBox> (类型擦除存储)
```

## 主要差异

| 特性 | DistEngine (C++) | DistRender (Rust) |
|-----|-----------------|-------------------|
| 继承 | 类继承 | Trait 实现 |
| 组件存储 | `vector<Component*>` | `Vec<Box<dyn Any>>` |
| 类型转换 | `static_cast<T*>` | `downcast_ref/mut<T>()` |
| 内存管理 | 手动 delete | 自动 Drop |
| 所有权 | 指针 | Box 独占所有权 |

## 使用示例

### 1. 创建 GameObject

```rust
use dist_render::component::{GameObject, Transform, Camera};

// 基础创建
let mut player = GameObject::new("Player");

// 便捷方法
let camera_obj = GameObject::with_camera("MainCamera");
let enemy = GameObject::with_transform("Enemy");
```

### 2. 添加组件

```rust
// 添加 Transform
player.add_component(Transform::new("PlayerTransform"));

// 添加 Camera
player.add_component(Camera::main_camera());
```

### 3. 查询组件

```rust
// 获取不可变引用
if let Some(transform) = player.get_component::<Transform>() {
    println!("Position: {:?}", transform.position);
}

// 获取可变引用
if let Some(camera) = player.get_component_mut::<Camera>() {
    camera.set_position(Vector3::new(0.0, 5.0, 10.0));
}

// 检查是否存在
if player.has_component::<Transform>() {
    println!("Has Transform!");
}
```

### 4. 移除组件

```rust
// 移除单个
let removed = player.remove_component::<Camera>();

// 移除所有同类型
let count = player.remove_all_components::<Transform>();

// 清除所有
player.clear_components();
```

### 5. 更新循环

```rust
for frame in 1..=60 {
    let delta_time = 0.016; // 60 FPS

    // 更新所有游戏对象
    for obj in scene.iter_mut() {
        obj.tick(delta_time);
    }
}
```

## Transform 组件

### 功能

- 位置、旋转（欧拉角）、缩放
- 世界矩阵计算（脏标记优化）
- 四元数支持
- 前向向量计算

### 示例

```rust
let mut transform = Transform::new("MyTransform");

// 设置位置
transform.set_position(Vector3::new(1.0, 2.0, 3.0));

// 设置旋转（度数）
transform.set_euler_angle(Vector3::new(45.0, 0.0, 0.0));

// 设置缩放
transform.set_scale(Vector3::new(2.0, 2.0, 2.0));

// 获取世界矩阵
let world = transform.world_matrix();
```

## Camera 组件

### 功能

- 相机坐标系（Right, Up, Look）
- 透视投影设置
- 视图矩阵和投影矩阵
- 移动（Strafe, Walk）
- 旋转（Pitch, RotateY）
- LookAt 功能

### 示例

```rust
let mut camera = Camera::main_camera();

// 设置透视投影
camera.set_lens(
    60.0 * PI / 180.0,  // FOV (弧度)
    16.0 / 9.0,         // 宽高比
    0.1,                // 近平面
    1000.0              // 远平面
);

// LookAt 目标
camera.look_at(
    Vector3::new(0.0, 5.0, 10.0),  // 相机位置
    Vector3::new(0.0, 0.0, 0.0),   // 目标位置
    Vector3::new(0.0, 1.0, 0.0)    // 世界上向量
);

// 移动
camera.walk(5.0);     // 向前
camera.strafe(-2.0);  // 向左

// 旋转
camera.rotate_y(PI / 6.0);  // 绕 Y 轴旋转 30 度
camera.pitch(PI / 12.0);    // 俯仰 15 度

// 获取矩阵
let view = camera.view_matrix();
let proj = camera.proj_matrix();
```

## GameObject 组件

### 功能

- 组件容器和管理
- 类型安全的组件查询
- 自动内存管理
- 启用/禁用状态
- 组件生命周期管理

### API

```rust
// 基础操作
add_component<T>(component: T)
remove_component<T>() -> bool
has_component<T>() -> bool

// 查询
get_component<T>() -> Option<&T>
get_component_mut<T>() -> Option<&mut T>
get_components<T>() -> Vec<&T>
get_component_at<T>(index) -> Option<&T>

// 统计
component_count() -> usize
component_count_of_type<T>() -> usize

// 管理
clear_components()
remove_all_components<T>() -> usize
```

## 运行示例

```bash
# Camera 组件示例
cargo run --example camera_demo

# GameObject 组件系统示例
cargo run --example game_object_demo
```

## 测试

```bash
# 运行所有组件测试
cargo test --lib component

# 运行特定组件测试
cargo test --lib component::game_object
```

## 设计模式

### 1. 组件模式 (Component Pattern)
游戏对象通过组合不同的组件来获得功能，而不是通过继承。

### 2. 脏标记模式 (Dirty Flag Pattern)
Transform 和 Camera 使用脏标记来延迟计算矩阵，只在需要时才更新。

### 3. 类型擦除 (Type Erasure)
GameObject 使用 `Box<dyn Any>` 来存储不同类型的组件。

### 4. RAII (Resource Acquisition Is Initialization)
Rust 的所有权系统自动管理组件的生命周期，无需手动释放内存。

## 性能考虑

1. **脏标记优化** - 矩阵只在必要时重新计算
2. **类型 ID 缓存** - 使用 `TypeId` 进行快速类型比较
3. **零成本抽象** - Component trait 的 `tick` 方法可以被内联
4. **所有权语义** - 避免运行时引用计数开销

## 未来扩展

可能的扩展方向：

- [ ] 支持场景图（Scene Graph）
- [ ] 组件依赖系统
- [ ] 序列化/反序列化支持
- [ ] 组件事件系统
- [ ] ECS (Entity Component System) 架构
- [ ] 组件查询优化（类型映射表）

## 参考

- DistEngine Component 系统
- Unity GameObject 和 Component 模型
- Bevy ECS 架构
