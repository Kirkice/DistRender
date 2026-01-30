//! 描述符管理模块
//!
//! 提供统一的描述符堆和描述符集管理接口，用于管理 GPU 资源视图。
//! 借鉴 DistEngine 的描述符堆设计，提供高效的描述符分配和管理。
//!
//! # 设计原则
//!
//! - **统一抽象**：为 DX12 和 Vulkan 提供统一的描述符管理接口
//! - **类型安全**：使用 Rust 类型系统确保描述符类型安全
//! - **自动管理**：使用 RAII 模式自动管理描述符生命周期
//! - **高效分配**：支持批量分配和预分配策略
//!
//! # DirectX 12 描述符类型
//!
//! - **RTV** (Render Target View)：渲染目标视图，用于渲染输出
//! - **DSV** (Depth Stencil View)：深度模板视图，用于深度测试
//! - **CBV** (Constant Buffer View)：常量缓冲视图，用于着色器常量
//! - **SRV** (Shader Resource View)：着色资源视图，用于着色器读取纹理/缓冲
//! - **UAV** (Unordered Access View)：无序访问视图，用于计算着色器读写
//!
//! # Vulkan 描述符类型
//!
//! - **Uniform Buffer**：对应 CBV
//! - **Storage Buffer**：对应 UAV
//! - **Sampled Image**：对应 SRV
//! - **Storage Image**：对应 UAV (图像)
//! - **Input Attachment**：用于渲染通道输入

use crate::core::error::{Result, DistRenderError};
use std::collections::HashMap;

/// 描述符类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorType {
    /// 渲染目标视图 (RTV)
    RenderTargetView,
    /// 深度模板视图 (DSV)
    DepthStencilView,
    /// 常量缓冲视图 (CBV)
    ConstantBufferView,
    /// 着色资源视图 (SRV)
    ShaderResourceView,
    /// 无序访问视图 (UAV)
    UnorderedAccessView,
    /// 采样器
    Sampler,
}

impl DescriptorType {
    /// 描述符类型是否需要着色器可见
    pub fn is_shader_visible(&self) -> bool {
        matches!(
            self,
            DescriptorType::ConstantBufferView
                | DescriptorType::ShaderResourceView
                | DescriptorType::UnorderedAccessView
                | DescriptorType::Sampler
        )
    }

    /// 获取描述符类型名称
    pub fn name(&self) -> &'static str {
        match self {
            DescriptorType::RenderTargetView => "RTV",
            DescriptorType::DepthStencilView => "DSV",
            DescriptorType::ConstantBufferView => "CBV",
            DescriptorType::ShaderResourceView => "SRV",
            DescriptorType::UnorderedAccessView => "UAV",
            DescriptorType::Sampler => "Sampler",
        }
    }
}

/// 描述符堆描述信息
#[derive(Debug, Clone)]
pub struct DescriptorHeapDescriptor {
    /// 描述符类型
    pub descriptor_type: DescriptorType,
    /// 描述符数量
    pub num_descriptors: u32,
    /// 是否着色器可见
    pub shader_visible: bool,
    /// 调试名称
    pub name: Option<String>,
}

impl DescriptorHeapDescriptor {
    /// 创建新的描述符堆描述符
    pub fn new(descriptor_type: DescriptorType, num_descriptors: u32) -> Self {
        Self {
            descriptor_type,
            num_descriptors,
            shader_visible: descriptor_type.is_shader_visible(),
            name: None,
        }
    }

    /// 设置调试名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 设置着色器可见性
    pub fn with_shader_visible(mut self, visible: bool) -> Self {
        self.shader_visible = visible;
        self
    }

    /// 创建 RTV 堆描述符
    pub fn rtv(num_descriptors: u32) -> Self {
        Self::new(DescriptorType::RenderTargetView, num_descriptors)
            .with_name("RTV Heap")
    }

    /// 创建 DSV 堆描述符
    pub fn dsv(num_descriptors: u32) -> Self {
        Self::new(DescriptorType::DepthStencilView, num_descriptors)
            .with_name("DSV Heap")
    }

    /// 创建 SRV/CBV/UAV 堆描述符
    pub fn srv_cbv_uav(num_descriptors: u32) -> Self {
        Self::new(DescriptorType::ShaderResourceView, num_descriptors)
            .with_shader_visible(true)
            .with_name("SRV/CBV/UAV Heap")
    }

    /// 创建采样器堆描述符
    pub fn sampler(num_descriptors: u32) -> Self {
        Self::new(DescriptorType::Sampler, num_descriptors)
            .with_shader_visible(true)
            .with_name("Sampler Heap")
    }
}

/// 描述符句柄（CPU 可见）
#[derive(Debug, Clone, Copy)]
pub struct CpuDescriptorHandle {
    /// 句柄指针值
    pub ptr: usize,
    /// 描述符索引
    pub index: u32,
}

impl CpuDescriptorHandle {
    /// 创建新的 CPU 描述符句柄
    pub fn new(ptr: usize, index: u32) -> Self {
        Self { ptr, index }
    }

    /// 偏移句柄
    pub fn offset(&self, count: u32, increment_size: u32) -> Self {
        Self {
            ptr: self.ptr + (count * increment_size) as usize,
            index: self.index + count,
        }
    }
}

/// 描述符句柄（GPU 可见）
#[derive(Debug, Clone, Copy)]
pub struct GpuDescriptorHandle {
    /// 句柄指针值
    pub ptr: u64,
    /// 描述符索引
    pub index: u32,
}

impl GpuDescriptorHandle {
    /// 创建新的 GPU 描述符句柄
    pub fn new(ptr: u64, index: u32) -> Self {
        Self { ptr, index }
    }

    /// 偏移句柄
    pub fn offset(&self, count: u32, increment_size: u32) -> Self {
        Self {
            ptr: self.ptr + (count * increment_size) as u64,
            index: self.index + count,
        }
    }
}

/// 描述符句柄对（CPU + GPU）
#[derive(Debug, Clone, Copy)]
pub struct DescriptorHandle {
    /// CPU 可见句柄
    pub cpu: CpuDescriptorHandle,
    /// GPU 可见句柄（仅对着色器可见的堆有效）
    pub gpu: Option<GpuDescriptorHandle>,
}

impl DescriptorHandle {
    /// 创建新的描述符句柄对
    pub fn new(cpu: CpuDescriptorHandle, gpu: Option<GpuDescriptorHandle>) -> Self {
        Self { cpu, gpu }
    }

    /// 偏移句柄对
    pub fn offset(&self, count: u32, increment_size: u32) -> Self {
        Self {
            cpu: self.cpu.offset(count, increment_size),
            gpu: self.gpu.map(|g| g.offset(count, increment_size)),
        }
    }
}

/// 描述符分配器
///
/// 管理描述符的分配和释放，支持线性分配策略。
/// 借鉴 DistEngine 的 RenderTargetManager 设计。
pub struct DescriptorAllocator {
    /// 描述符类型
    descriptor_type: DescriptorType,
    /// 最大描述符数量
    max_descriptors: u32,
    /// 当前已分配数量
    allocated_count: u32,
    /// 是否着色器可见
    shader_visible: bool,
    /// 描述符增量大小
    increment_size: u32,
    /// 已分配的描述符映射（ID -> 句柄）
    descriptors: HashMap<u64, DescriptorHandle>,
}

impl DescriptorAllocator {
    /// 创建新的描述符分配器
    pub fn new(
        descriptor_type: DescriptorType,
        max_descriptors: u32,
        shader_visible: bool,
        increment_size: u32,
    ) -> Self {
        Self {
            descriptor_type,
            max_descriptors,
            allocated_count: 0,
            shader_visible,
            increment_size,
            descriptors: HashMap::new(),
        }
    }

    /// 分配描述符
    ///
    /// # 参数
    ///
    /// * `id` - 描述符唯一标识符
    /// * `cpu_base` - CPU 句柄基地址
    /// * `gpu_base` - GPU 句柄基地址（可选）
    ///
    /// # 返回值
    ///
    /// 返回分配的描述符句柄
    pub fn allocate(
        &mut self,
        id: u64,
        cpu_base: usize,
        gpu_base: Option<u64>,
    ) -> Result<DescriptorHandle> {
        // 检查预算
        if self.allocated_count >= self.max_descriptors {
            return Err(DistRenderError::Runtime(format!(
                "Descriptor allocator out of budget: {}/{} for {}",
                self.allocated_count,
                self.max_descriptors,
                self.descriptor_type.name()
            )));
        }

        // 检查是否已存在
        if self.descriptors.contains_key(&id) {
            return Err(DistRenderError::Runtime(format!(
                "Descriptor with ID {} already exists",
                id
            )));
        }

        // 计算句柄
        let index = self.allocated_count;
        let cpu = CpuDescriptorHandle::new(
            cpu_base + (index * self.increment_size) as usize,
            index,
        );
        let gpu = gpu_base.map(|base| {
            GpuDescriptorHandle::new(base + (index * self.increment_size) as u64, index)
        });

        let handle = DescriptorHandle::new(cpu, gpu);
        self.descriptors.insert(id, handle);
        self.allocated_count += 1;

        Ok(handle)
    }

    /// 获取描述符句柄
    pub fn get(&self, id: u64) -> Option<&DescriptorHandle> {
        self.descriptors.get(&id)
    }

    /// 释放描述符
    pub fn free(&mut self, id: u64) -> bool {
        self.descriptors.remove(&id).is_some()
    }

    /// 获取已分配数量
    pub fn allocated_count(&self) -> u32 {
        self.allocated_count
    }

    /// 获取最大容量
    pub fn capacity(&self) -> u32 {
        self.max_descriptors
    }

    /// 是否已满
    pub fn is_full(&self) -> bool {
        self.allocated_count >= self.max_descriptors
    }

    /// 获取描述符类型
    pub fn descriptor_type(&self) -> DescriptorType {
        self.descriptor_type
    }
}

/// 描述符堆统计信息
#[derive(Debug, Clone)]
pub struct DescriptorHeapStats {
    /// 描述符类型
    pub descriptor_type: DescriptorType,
    /// 总容量
    pub capacity: u32,
    /// 已使用数量
    pub used: u32,
    /// 可用数量
    pub available: u32,
    /// 使用率 (0.0 - 1.0)
    pub usage_ratio: f32,
}

impl DescriptorHeapStats {
    /// 创建新的统计信息
    pub fn new(descriptor_type: DescriptorType, capacity: u32, used: u32) -> Self {
        let available = capacity.saturating_sub(used);
        let usage_ratio = if capacity > 0 {
            used as f32 / capacity as f32
        } else {
            0.0
        };

        Self {
            descriptor_type,
            capacity,
            used,
            available,
            usage_ratio,
        }
    }
}

/// 描述符管理器
///
/// 管理多个描述符堆和分配器，提供统一的描述符管理接口。
pub struct DescriptorManager {
    /// RTV 分配器
    rtv_allocator: Option<DescriptorAllocator>,
    /// DSV 分配器
    dsv_allocator: Option<DescriptorAllocator>,
    /// SRV/CBV/UAV 分配器
    srv_cbv_uav_allocator: Option<DescriptorAllocator>,
    /// 采样器分配器
    sampler_allocator: Option<DescriptorAllocator>,
}

impl DescriptorManager {
    /// 创建新的描述符管理器
    pub fn new() -> Self {
        Self {
            rtv_allocator: None,
            dsv_allocator: None,
            srv_cbv_uav_allocator: None,
            sampler_allocator: None,
        }
    }

    /// 初始化 RTV 分配器
    pub fn init_rtv(&mut self, max_descriptors: u32, increment_size: u32) {
        self.rtv_allocator = Some(DescriptorAllocator::new(
            DescriptorType::RenderTargetView,
            max_descriptors,
            false,
            increment_size,
        ));
    }

    /// 初始化 DSV 分配器
    pub fn init_dsv(&mut self, max_descriptors: u32, increment_size: u32) {
        self.dsv_allocator = Some(DescriptorAllocator::new(
            DescriptorType::DepthStencilView,
            max_descriptors,
            false,
            increment_size,
        ));
    }

    /// 初始化 SRV/CBV/UAV 分配器
    pub fn init_srv_cbv_uav(&mut self, max_descriptors: u32, increment_size: u32) {
        self.srv_cbv_uav_allocator = Some(DescriptorAllocator::new(
            DescriptorType::ShaderResourceView,
            max_descriptors,
            true,
            increment_size,
        ));
    }

    /// 初始化采样器分配器
    pub fn init_sampler(&mut self, max_descriptors: u32, increment_size: u32) {
        self.sampler_allocator = Some(DescriptorAllocator::new(
            DescriptorType::Sampler,
            max_descriptors,
            true,
            increment_size,
        ));
    }

    /// 获取分配器（可变）
    fn get_allocator_mut(
        &mut self,
        descriptor_type: DescriptorType,
    ) -> Result<&mut DescriptorAllocator> {
        let allocator = match descriptor_type {
            DescriptorType::RenderTargetView => &mut self.rtv_allocator,
            DescriptorType::DepthStencilView => &mut self.dsv_allocator,
            DescriptorType::ConstantBufferView
            | DescriptorType::ShaderResourceView
            | DescriptorType::UnorderedAccessView => &mut self.srv_cbv_uav_allocator,
            DescriptorType::Sampler => &mut self.sampler_allocator,
        };

        allocator
            .as_mut()
            .ok_or_else(|| {
                DistRenderError::Runtime(format!(
                    "Descriptor allocator not initialized for {}",
                    descriptor_type.name()
                ))
            })
    }

    /// 获取分配器（不可变）
    fn get_allocator(
        &self,
        descriptor_type: DescriptorType,
    ) -> Result<&DescriptorAllocator> {
        let allocator = match descriptor_type {
            DescriptorType::RenderTargetView => &self.rtv_allocator,
            DescriptorType::DepthStencilView => &self.dsv_allocator,
            DescriptorType::ConstantBufferView
            | DescriptorType::ShaderResourceView
            | DescriptorType::UnorderedAccessView => &self.srv_cbv_uav_allocator,
            DescriptorType::Sampler => &self.sampler_allocator,
        };

        allocator
            .as_ref()
            .ok_or_else(|| {
                DistRenderError::Runtime(format!(
                    "Descriptor allocator not initialized for {}",
                    descriptor_type.name()
                ))
            })
    }

    /// 分配描述符
    pub fn allocate(
        &mut self,
        descriptor_type: DescriptorType,
        id: u64,
        cpu_base: usize,
        gpu_base: Option<u64>,
    ) -> Result<DescriptorHandle> {
        self.get_allocator_mut(descriptor_type)?
            .allocate(id, cpu_base, gpu_base)
    }

    /// 获取描述符句柄
    pub fn get(
        &self,
        descriptor_type: DescriptorType,
        id: u64,
    ) -> Result<DescriptorHandle> {
        self.get_allocator(descriptor_type)?
            .get(id)
            .copied()
            .ok_or_else(|| {
                DistRenderError::Runtime(format!(
                    "Descriptor with ID {} not found in {} allocator",
                    id,
                    descriptor_type.name()
                ))
            })
    }

    /// 释放描述符
    pub fn free(&mut self, descriptor_type: DescriptorType, id: u64) -> Result<()> {
        if self.get_allocator_mut(descriptor_type)?.free(id) {
            Ok(())
        } else {
            Err(DistRenderError::Runtime(format!(
                "Failed to free descriptor with ID {}",
                id
            )))
        }
    }

    /// 获取统计信息
    pub fn stats(&self, descriptor_type: DescriptorType) -> Result<DescriptorHeapStats> {
        let allocator = self.get_allocator(descriptor_type)?;
        Ok(DescriptorHeapStats::new(
            descriptor_type,
            allocator.capacity(),
            allocator.allocated_count(),
        ))
    }

    /// 获取所有统计信息
    pub fn all_stats(&self) -> Vec<DescriptorHeapStats> {
        let mut stats = Vec::new();

        if let Some(alloc) = &self.rtv_allocator {
            stats.push(DescriptorHeapStats::new(
                alloc.descriptor_type(),
                alloc.capacity(),
                alloc.allocated_count(),
            ));
        }

        if let Some(alloc) = &self.dsv_allocator {
            stats.push(DescriptorHeapStats::new(
                alloc.descriptor_type(),
                alloc.capacity(),
                alloc.allocated_count(),
            ));
        }

        if let Some(alloc) = &self.srv_cbv_uav_allocator {
            stats.push(DescriptorHeapStats::new(
                alloc.descriptor_type(),
                alloc.capacity(),
                alloc.allocated_count(),
            ));
        }

        if let Some(alloc) = &self.sampler_allocator {
            stats.push(DescriptorHeapStats::new(
                alloc.descriptor_type(),
                alloc.capacity(),
                alloc.allocated_count(),
            ));
        }

        stats
    }
}

impl Default for DescriptorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_type() {
        assert!(DescriptorType::ShaderResourceView.is_shader_visible());
        assert!(!DescriptorType::RenderTargetView.is_shader_visible());
        assert_eq!(DescriptorType::RenderTargetView.name(), "RTV");
    }

    #[test]
    fn test_descriptor_heap_descriptor() {
        let desc = DescriptorHeapDescriptor::rtv(100);
        assert_eq!(desc.descriptor_type, DescriptorType::RenderTargetView);
        assert_eq!(desc.num_descriptors, 100);
        assert!(!desc.shader_visible);
        assert_eq!(desc.name, Some("RTV Heap".to_string()));

        let desc = DescriptorHeapDescriptor::srv_cbv_uav(128);
        assert!(desc.shader_visible);
    }

    #[test]
    fn test_cpu_descriptor_handle_offset() {
        let handle = CpuDescriptorHandle::new(1000, 0);
        let offset_handle = handle.offset(5, 32);
        assert_eq!(offset_handle.ptr, 1160); // 1000 + 5 * 32
        assert_eq!(offset_handle.index, 5);
    }

    #[test]
    fn test_gpu_descriptor_handle_offset() {
        let handle = GpuDescriptorHandle::new(2000, 0);
        let offset_handle = handle.offset(10, 32);
        assert_eq!(offset_handle.ptr, 2320); // 2000 + 10 * 32
        assert_eq!(offset_handle.index, 10);
    }

    #[test]
    fn test_descriptor_allocator() {
        let mut allocator = DescriptorAllocator::new(
            DescriptorType::RenderTargetView,
            10,
            false,
            32,
        );

        // 分配描述符
        let handle = allocator.allocate(0, 1000, None).unwrap();
        assert_eq!(handle.cpu.ptr, 1000);
        assert_eq!(handle.cpu.index, 0);
        assert!(handle.gpu.is_none());

        // 分配第二个
        let handle2 = allocator.allocate(1, 1000, None).unwrap();
        assert_eq!(handle2.cpu.ptr, 1032); // 1000 + 32
        assert_eq!(handle2.cpu.index, 1);

        // 获取描述符
        let retrieved = allocator.get(0).unwrap();
        assert_eq!(retrieved.cpu.ptr, 1000);

        // 统计信息
        assert_eq!(allocator.allocated_count(), 2);
        assert_eq!(allocator.capacity(), 10);
        assert!(!allocator.is_full());

        // 释放描述符
        assert!(allocator.free(0));
        assert!(allocator.get(0).is_none());
    }

    #[test]
    fn test_descriptor_allocator_budget() {
        let mut allocator = DescriptorAllocator::new(
            DescriptorType::RenderTargetView,
            2,
            false,
            32,
        );

        allocator.allocate(0, 1000, None).unwrap();
        allocator.allocate(1, 1000, None).unwrap();

        // 应该超出预算
        let result = allocator.allocate(2, 1000, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_descriptor_manager() {
        let mut manager = DescriptorManager::new();

        // 初始化分配器
        manager.init_rtv(100, 32);
        manager.init_dsv(10, 32);
        manager.init_srv_cbv_uav(128, 32);

        // 分配 RTV
        let handle = manager
            .allocate(DescriptorType::RenderTargetView, 0, 1000, None)
            .unwrap();
        assert_eq!(handle.cpu.ptr, 1000);

        // 获取 RTV
        let retrieved = manager
            .get(DescriptorType::RenderTargetView, 0)
            .unwrap();
        assert_eq!(retrieved.cpu.ptr, 1000);

        // 统计信息
        let stats = manager.stats(DescriptorType::RenderTargetView).unwrap();
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.used, 1);
        assert_eq!(stats.available, 99);

        // 释放
        manager.free(DescriptorType::RenderTargetView, 0).unwrap();
    }

    #[test]
    fn test_descriptor_heap_stats() {
        let stats = DescriptorHeapStats::new(DescriptorType::RenderTargetView, 100, 50);
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.used, 50);
        assert_eq!(stats.available, 50);
        assert_eq!(stats.usage_ratio, 0.5);
    }
}
