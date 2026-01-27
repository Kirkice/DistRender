//! 资源管理模块
//!
//! 提供统一的资源管理接口，用于管理缓冲区、纹理等GPU资源。
//! 借鉴 DistEngine 的 UploadBuffer 和资源管理设计。
//!
//! # 设计原则
//!
//! - **统一接口**：为不同图形API提供统一的资源管理抽象
//! - **自动对齐**：自动处理常量缓冲区的对齐要求
//! - **生命周期管理**：使用Rust所有权系统自动管理资源生命周期
//! - **类型安全**：通过泛型和trait约束保证类型安全

use crate::core::error::{Result, DistRenderError, GraphicsError};
use std::marker::PhantomData;

/// 缓冲区使用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferUsageType {
    /// 顶点缓冲区
    Vertex,
    /// 索引缓冲区
    Index,
    /// 常量缓冲区（Uniform Buffer）
    Constant,
    /// 存储缓冲区（Storage Buffer）
    Storage,
    /// 上传缓冲区（CPU -> GPU）
    Upload,
    /// 读回缓冲区（GPU -> CPU）
    ReadBack,
}

/// 缓冲区内存类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// GPU本地内存（最快，仅GPU可访问）
    DeviceLocal,
    /// CPU可见内存（CPU和GPU都可访问）
    HostVisible,
    /// CPU缓存一致性内存（更新频率高）
    HostCoherent,
}

/// 缓冲区描述信息
#[derive(Debug, Clone)]
pub struct BufferDescriptor {
    /// 缓冲区大小（字节）
    pub size: u64,
    /// 使用类型
    pub usage: BufferUsageType,
    /// 内存类型
    pub memory_type: MemoryType,
    /// 调试名称（可选）
    pub name: Option<String>,
}

impl BufferDescriptor {
    /// 创建新的缓冲区描述符
    pub fn new(size: u64, usage: BufferUsageType, memory_type: MemoryType) -> Self {
        Self {
            size,
            usage,
            memory_type,
            name: None,
        }
    }

    /// 设置调试名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 计算对齐后的大小（DirectX 12 常量缓冲区要求256字节对齐）
    pub fn aligned_size(&self) -> u64 {
        if self.usage == BufferUsageType::Constant {
            // 常量缓冲区对齐到256字节边界
            (self.size + 255) & !255
        } else {
            self.size
        }
    }
}

/// 上传缓冲区（CPU -> GPU）
///
/// 类似于 DistEngine 的 UploadBuffer<T>，用于频繁更新的数据。
/// 使用泛型确保类型安全，自动处理内存对齐。
///
/// # 类型参数
///
/// * `T` - 缓冲区中存储的数据类型
///
/// # 示例
///
/// ```rust
/// // 创建常量缓冲区
/// let mut buffer = UploadBuffer::<ObjectConstants>::new(
///     device,
///     16, // 最多16个对象
///     BufferUsageType::Constant
/// )?;
///
/// // 更新数据
/// let constants = ObjectConstants { ... };
/// buffer.copy_data(0, &constants)?;
/// ```
pub struct UploadBuffer<T> {
    /// 元素数量
    element_count: usize,
    /// 每个元素的大小（对齐后）
    element_size: u64,
    /// 总大小
    total_size: u64,
    /// 使用类型
    usage: BufferUsageType,
    /// 幻影数据，用于类型参数
    _phantom: PhantomData<T>,
}

impl<T> UploadBuffer<T> {
    /// 创建新的上传缓冲区
    ///
    /// # 参数
    ///
    /// * `element_count` - 元素数量
    /// * `usage` - 缓冲区使用类型
    ///
    /// # 返回值
    ///
    /// 返回新创建的上传缓冲区
    pub fn new(element_count: usize, usage: BufferUsageType) -> Self {
        let element_size = std::mem::size_of::<T>() as u64;

        // 如果是常量缓冲区，对齐到256字节
        let aligned_element_size = if usage == BufferUsageType::Constant {
            (element_size + 255) & !255
        } else {
            element_size
        };

        let total_size = aligned_element_size * element_count as u64;

        Self {
            element_count,
            element_size: aligned_element_size,
            total_size,
            usage,
            _phantom: PhantomData,
        }
    }

    /// 获取元素数量
    pub fn element_count(&self) -> usize {
        self.element_count
    }

    /// 获取每个元素的大小（对齐后）
    pub fn element_size(&self) -> u64 {
        self.element_size
    }

    /// 获取总大小
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// 获取使用类型
    pub fn usage(&self) -> BufferUsageType {
        self.usage
    }

    /// 计算元素在缓冲区中的偏移量
    pub fn element_offset(&self, index: usize) -> u64 {
        assert!(index < self.element_count, "Index out of bounds");
        self.element_size * index as u64
    }

    /// 创建缓冲区描述符
    pub fn descriptor(&self, name: Option<String>) -> BufferDescriptor {
        BufferDescriptor {
            size: self.total_size,
            usage: self.usage,
            memory_type: MemoryType::HostVisible,
            name,
        }
    }
}

/// 纹理描述信息
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    /// 宽度
    pub width: u32,
    /// 高度
    pub height: u32,
    /// 深度（3D纹理）或数组层数
    pub depth_or_array_layers: u32,
    /// Mip等级数量
    pub mip_levels: u32,
    /// 纹理格式
    pub format: TextureFormat,
    /// 纹理类型
    pub texture_type: TextureType,
    /// 调试名称
    pub name: Option<String>,
}

/// 纹理格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// RGBA 8位无符号整数
    Rgba8Unorm,
    /// RGBA 8位sRGB
    Rgba8Srgb,
    /// BGRA 8位无符号整数
    Bgra8Unorm,
    /// R 32位浮点
    R32Float,
    /// RGBA 32位浮点
    Rgba32Float,
    /// 深度 24位 + 模板 8位
    Depth24PlusStencil8,
    /// 深度 32位浮点
    Depth32Float,
}

/// 纹理类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureType {
    /// 1D纹理
    Texture1D,
    /// 2D纹理
    Texture2D,
    /// 3D纹理
    Texture3D,
    /// 立方体贴图
    TextureCube,
}

/// 帧资源
///
/// 借鉴 DistEngine 的三缓冲设计，每帧都有独立的资源集。
/// 这样可以避免GPU和CPU之间的同步等待，提高性能。
///
/// # 设计说明
///
/// 使用三个帧资源循环使用：
/// - 帧 N: CPU正在写入
/// - 帧 N-1: GPU正在处理
/// - 帧 N-2: 完成，可以复用
pub struct FrameResource {
    /// 帧索引
    pub frame_index: usize,
    /// Fence值，用于同步
    pub fence_value: u64,
    /// 资源是否可用
    pub available: bool,
}

impl FrameResource {
    /// 创建新的帧资源
    pub fn new(frame_index: usize) -> Self {
        Self {
            frame_index,
            fence_value: 0,
            available: true,
        }
    }

    /// 标记为不可用（GPU正在使用）
    pub fn mark_in_use(&mut self, fence_value: u64) {
        self.available = false;
        self.fence_value = fence_value;
    }

    /// 标记为可用
    pub fn mark_available(&mut self) {
        self.available = true;
    }
}

/// 帧资源池
///
/// 管理多个帧资源的循环使用。
/// 默认使用3个帧资源（三缓冲）。
pub struct FrameResourcePool {
    /// 帧资源列表
    resources: Vec<FrameResource>,
    /// 当前帧索引
    current_index: usize,
    /// 帧资源数量
    count: usize,
}

impl FrameResourcePool {
    /// 创建新的帧资源池
    ///
    /// # 参数
    ///
    /// * `count` - 帧资源数量（通常为2或3）
    pub fn new(count: usize) -> Self {
        assert!(count >= 2, "At least 2 frame resources required");

        let resources = (0..count)
            .map(|i| FrameResource::new(i))
            .collect();

        Self {
            resources,
            current_index: 0,
            count,
        }
    }

    /// 创建默认的三缓冲资源池
    pub fn triple_buffering() -> Self {
        Self::new(3)
    }

    /// 创建双缓冲资源池
    pub fn double_buffering() -> Self {
        Self::new(2)
    }

    /// 获取当前帧资源
    pub fn current(&self) -> &FrameResource {
        &self.resources[self.current_index]
    }

    /// 获取当前帧资源的可变引用
    pub fn current_mut(&mut self) -> &mut FrameResource {
        &mut self.resources[self.current_index]
    }

    /// 根据索引获取帧资源
    pub fn get(&self, index: usize) -> Option<&FrameResource> {
        self.resources.get(index)
    }

    /// 根据索引获取帧资源的可变引用
    pub fn get_mut(&mut self, index: usize) -> Option<&mut FrameResource> {
        self.resources.get_mut(index)
    }

    /// 获取当前帧索引
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// 移动到下一帧
    pub fn advance(&mut self) -> &FrameResource {
        self.current_index = (self.current_index + 1) % self.count;
        self.current()
    }

    /// 根据Fence值更新帧资源可用性
    pub fn update_availability(&mut self, completed_fence_value: u64) {
        for resource in &mut self.resources {
            if !resource.available && resource.fence_value <= completed_fence_value {
                resource.mark_available();
            }
        }
    }

    /// 等待下一个可用的帧资源
    ///
    /// 返回下一个可用帧资源的Fence值（如果需要等待）
    pub fn next_available_fence_value(&self) -> Option<u64> {
        let next_index = (self.current_index + 1) % self.count;
        let next_resource = &self.resources[next_index];

        if next_resource.available {
            None // 已经可用，无需等待
        } else {
            Some(next_resource.fence_value) // 需要等待到这个Fence值
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_descriptor_alignment() {
        let desc = BufferDescriptor::new(100, BufferUsageType::Constant, MemoryType::HostVisible);
        assert_eq!(desc.aligned_size(), 256); // 对齐到256字节

        let desc2 = BufferDescriptor::new(300, BufferUsageType::Constant, MemoryType::HostVisible);
        assert_eq!(desc2.aligned_size(), 512); // 对齐到512字节

        let desc3 = BufferDescriptor::new(100, BufferUsageType::Vertex, MemoryType::DeviceLocal);
        assert_eq!(desc3.aligned_size(), 100); // 顶点缓冲区不需要对齐
    }

    #[test]
    fn test_upload_buffer_sizing() {
        struct TestData {
            value: f32,
        }

        let buffer = UploadBuffer::<TestData>::new(10, BufferUsageType::Constant);
        assert_eq!(buffer.element_count(), 10);
        assert_eq!(buffer.element_size(), 256); // 对齐到256字节
        assert_eq!(buffer.total_size(), 2560); // 10 * 256

        let buffer2 = UploadBuffer::<TestData>::new(10, BufferUsageType::Vertex);
        assert_eq!(buffer2.element_size(), 4); // float大小，无需对齐
        assert_eq!(buffer2.total_size(), 40); // 10 * 4
    }

    #[test]
    fn test_frame_resource_pool() {
        let mut pool = FrameResourcePool::triple_buffering();

        assert_eq!(pool.current_index(), 0);
        assert!(pool.current().available);

        // 标记当前帧为使用中
        pool.current_mut().mark_in_use(1);
        assert!(!pool.current().available);

        // 前进到下一帧
        pool.advance();
        assert_eq!(pool.current_index(), 1);

        // 前进两次回到帧0
        pool.advance();
        pool.advance();
        assert_eq!(pool.current_index(), 0);

        // 模拟GPU完成
        pool.update_availability(1);
        assert!(pool.resources[0].available);
    }

    #[test]
    fn test_frame_resource_cycling() {
        let mut pool = FrameResourcePool::new(3);

        // 模拟渲染循环
        for frame in 0..10 {
            let fence = frame as u64;
            pool.current_mut().mark_in_use(fence);
            pool.advance();

            // 模拟GPU完成前几帧
            if frame >= 2 {
                pool.update_availability(fence - 2);
            }
        }

        // 检查所有资源状态
        pool.update_availability(10);
        for resource in &pool.resources {
            assert!(resource.available);
        }
    }
}
