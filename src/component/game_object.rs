//! GameObject 组件容器
//!
//! 参考 DistEngine 的 GameObject 类实现
//! 管理游戏对象及其附加的组件

use super::{Component, Transform, Camera};
use std::any::{Any, TypeId};

/// 组件存储包装器
///
/// 用于在 GameObject 中存储不同类型的组件
struct ComponentBox {
    /// 组件名称
    name: String,
    /// 组件实例（使用 Box<dyn Any> 实现类型擦除）
    component: Box<dyn Any>,
    /// 组件类型 ID
    type_id: TypeId,
}

/// GameObject - 游戏对象
///
/// 作为组件容器，可以添加、移除和查询组件
pub struct GameObject {
    /// 游戏对象名称
    name: String,

    /// 是否启用
    pub enabled: bool,

    /// 附加的组件列表
    components: Vec<ComponentBox>,
}

impl GameObject {
    /// 创建新的 GameObject
    ///
    /// # 参数
    /// - `name`: 游戏对象名称
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            components: Vec::new(),
        }
    }

    /// 设置名称
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// 获取名称
    pub fn get_name(&self) -> &str {
        &self.name
    }

    // ========== 组件管理 ==========

    /// 添加组件
    ///
    /// # 参数
    /// - `component`: 要添加的组件
    ///
    /// # 示例
    /// ```
    /// use dist_render::component::{GameObject, Transform};
    ///
    /// let mut go = GameObject::new("Player");
    /// go.add_component(Transform::new("PlayerTransform"));
    /// ```
    pub fn add_component<T: 'static>(&mut self, component: T) {
        let type_id = TypeId::of::<T>();
        let name = format!("{:?}", type_id); // 简化的名称

        self.components.push(ComponentBox {
            name,
            component: Box::new(component),
            type_id,
        });
    }

    /// 移除组件（按类型）
    ///
    /// # 返回
    /// 如果找到并移除了组件，返回 `true`；否则返回 `false`
    pub fn remove_component<T: 'static>(&mut self) -> bool {
        let type_id = TypeId::of::<T>();

        if let Some(index) = self.components.iter().position(|c| c.type_id == type_id) {
            self.components.remove(index);
            true
        } else {
            false
        }
    }

    /// 移除所有指定类型的组件
    ///
    /// # 返回
    /// 移除的组件数量
    pub fn remove_all_components<T: 'static>(&mut self) -> usize {
        let type_id = TypeId::of::<T>();
        let original_len = self.components.len();

        self.components.retain(|c| c.type_id != type_id);

        original_len - self.components.len()
    }

    /// 获取组件的不可变引用
    ///
    /// # 返回
    /// 如果找到了指定类型的组件，返回 `Some(&T)`；否则返回 `None`
    ///
    /// # 示例
    /// ```
    /// use dist_render::component::{GameObject, Transform};
    ///
    /// let mut go = GameObject::new("Player");
    /// go.add_component(Transform::new("PlayerTransform"));
    ///
    /// if let Some(transform) = go.get_component::<Transform>() {
    ///     println!("Position: {:?}", transform.position);
    /// }
    /// ```
    pub fn get_component<T: 'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        self.components
            .iter()
            .find(|c| c.type_id == type_id)
            .and_then(|c| c.component.downcast_ref::<T>())
    }

    /// 获取组件的可变引用
    ///
    /// # 返回
    /// 如果找到了指定类型的组件，返回 `Some(&mut T)`；否则返回 `None`
    ///
    /// # 示例
    /// ```
    /// use dist_render::component::{GameObject, Transform};
    /// use dist_render::core::math::Vector3;
    ///
    /// let mut go = GameObject::new("Player");
    /// go.add_component(Transform::new("PlayerTransform"));
    ///
    /// if let Some(transform) = go.get_component_mut::<Transform>() {
    ///     transform.set_position(Vector3::new(1.0, 0.0, 0.0));
    /// }
    /// ```
    pub fn get_component_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();

        self.components
            .iter_mut()
            .find(|c| c.type_id == type_id)
            .and_then(|c| c.component.downcast_mut::<T>())
    }

    /// 获取指定索引的组件（按类型）
    ///
    /// # 参数
    /// - `index`: 组件索引（从 0 开始）
    ///
    /// # 返回
    /// 如果找到了指定索引的组件，返回 `Some(&T)`；否则返回 `None`
    pub fn get_component_at<T: 'static>(&self, index: usize) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        self.components
            .iter()
            .filter(|c| c.type_id == type_id)
            .nth(index)
            .and_then(|c| c.component.downcast_ref::<T>())
    }

    /// 获取指定索引的组件的可变引用（按类型）
    pub fn get_component_at_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();

        self.components
            .iter_mut()
            .filter(|c| c.type_id == type_id)
            .nth(index)
            .and_then(|c| c.component.downcast_mut::<T>())
    }

    /// 获取所有指定类型的组件
    ///
    /// # 返回
    /// 包含所有匹配类型组件的不可变引用的 Vec
    pub fn get_components<T: 'static>(&self) -> Vec<&T> {
        let type_id = TypeId::of::<T>();

        self.components
            .iter()
            .filter(|c| c.type_id == type_id)
            .filter_map(|c| c.component.downcast_ref::<T>())
            .collect()
    }

    /// 获取所有指定类型的组件的可变引用
    pub fn get_components_mut<T: 'static>(&mut self) -> Vec<&mut T> {
        let type_id = TypeId::of::<T>();

        self.components
            .iter_mut()
            .filter(|c| c.type_id == type_id)
            .filter_map(|c| c.component.downcast_mut::<T>())
            .collect()
    }

    /// 检查是否有指定类型的组件
    pub fn has_component<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.components.iter().any(|c| c.type_id == type_id)
    }

    /// 获取组件数量
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// 获取指定类型的组件数量
    pub fn component_count_of_type<T: 'static>(&self) -> usize {
        let type_id = TypeId::of::<T>();
        self.components.iter().filter(|c| c.type_id == type_id).count()
    }

    /// 清除所有组件
    pub fn clear_components(&mut self) {
        self.components.clear();
    }

    // ========== 便捷方法 ==========

    /// 创建带有 Transform 组件的 GameObject
    pub fn with_transform(name: impl Into<String>) -> Self {
        let mut go = Self::new(name);
        go.add_component(Transform::default());
        go
    }

    /// 创建带有 Camera 组件的 GameObject
    pub fn with_camera(name: impl Into<String>) -> Self {
        let mut go = Self::new(name);
        go.add_component(Camera::default());
        go
    }

    /// 获取或添加 Transform 组件
    ///
    /// 如果已存在 Transform，返回其可变引用；否则添加一个新的并返回
    pub fn get_or_add_transform(&mut self) -> &mut Transform {
        if !self.has_component::<Transform>() {
            self.add_component(Transform::default());
        }
        self.get_component_mut::<Transform>().unwrap()
    }

    /// 获取或添加 Camera 组件
    pub fn get_or_add_camera(&mut self) -> &mut Camera {
        if !self.has_component::<Camera>() {
            self.add_component(Camera::default());
        }
        self.get_component_mut::<Camera>().unwrap()
    }
}

impl Component for GameObject {
    fn name(&self) -> &str {
        &self.name
    }

    fn tick(&mut self, delta_time: f32) {
        if !self.enabled {
            return;
        }

        // 更新所有组件
        // 注意：由于组件被存储为 Box<dyn Any>，我们需要特殊处理
        // 这里提供基础实现，具体组件的 tick 需要手动调用

        // 更新 Transform 组件
        for transform in self.get_components_mut::<Transform>() {
            transform.tick(delta_time);
        }

        // 更新 Camera 组件
        for camera in self.get_components_mut::<Camera>() {
            camera.tick(delta_time);
        }
    }
}

impl Default for GameObject {
    fn default() -> Self {
        Self::new("GameObject")
    }
}

// 实现 Drop trait 来清理组件（模拟 C++ 析构函数）
impl Drop for GameObject {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        {
            if !self.components.is_empty() {
                tracing::debug!(
                    "GameObject '{}' being dropped with {} components",
                    self.name,
                    self.components.len()
                );
            }
        }
        // Rust 会自动清理 Vec 和 Box，所以不需要手动删除
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_game_object() {
        let go = GameObject::new("TestObject");
        assert_eq!(go.get_name(), "TestObject");
        assert!(go.enabled);
        assert_eq!(go.component_count(), 0);
    }

    #[test]
    fn test_add_and_get_component() {
        let mut go = GameObject::new("TestObject");
        go.add_component(Transform::new("TestTransform"));

        assert!(go.has_component::<Transform>());
        assert_eq!(go.component_count(), 1);

        let transform = go.get_component::<Transform>();
        assert!(transform.is_some());
    }

    #[test]
    fn test_remove_component() {
        let mut go = GameObject::new("TestObject");
        go.add_component(Transform::new("TestTransform"));

        assert!(go.has_component::<Transform>());
        assert!(go.remove_component::<Transform>());
        assert!(!go.has_component::<Transform>());
    }

    #[test]
    fn test_multiple_components() {
        let mut go = GameObject::new("TestObject");
        go.add_component(Transform::new("Transform1"));
        go.add_component(Camera::new("Camera1"));

        assert_eq!(go.component_count(), 2);
        assert!(go.has_component::<Transform>());
        assert!(go.has_component::<Camera>());
    }
}
