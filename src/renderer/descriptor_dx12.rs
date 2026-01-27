//! DirectX 12 描述符堆实现
//!
//! 提供 DX12 特定的描述符堆管理功能。
//! 借鉴 DistEngine 的 DescriptorHeap 和 RenderTargetManager 设计。

use crate::core::error::{Result, DistRenderError, GraphicsError};
use crate::renderer::descriptor::{
    CpuDescriptorHandle, DescriptorHeapDescriptor, DescriptorManager, DescriptorType,
    GpuDescriptorHandle,
};
use std::sync::Arc;
use windows::Win32::Graphics::Direct3D12::*;

/// DX12 描述符堆
///
/// 封装 ID3D12DescriptorHeap 并提供类型安全的访问接口。
pub struct Dx12DescriptorHeap {
    /// 底层 DX12 描述符堆
    heap: ID3D12DescriptorHeap,
    /// 描述符类型
    descriptor_type: DescriptorType,
    /// 描述符增量大小
    increment_size: u32,
    /// CPU 句柄基址
    cpu_start: usize,
    /// GPU 句柄基址（仅对着色器可见的堆）
    gpu_start: Option<u64>,
    /// 描述符数量
    num_descriptors: u32,
    /// 是否着色器可见
    shader_visible: bool,
}

impl Dx12DescriptorHeap {
    /// 创建新的 DX12 描述符堆
    ///
    /// # 参数
    ///
    /// * `device` - DX12 设备
    /// * `desc` - 描述符堆描述信息
    ///
    /// # 返回值
    ///
    /// 返回新创建的描述符堆
    pub fn new(device: &ID3D12Device, desc: &DescriptorHeapDescriptor) -> Result<Self> {
        unsafe {
            // 转换描述符类型
            let heap_type = match desc.descriptor_type {
                DescriptorType::RenderTargetView => D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                DescriptorType::DepthStencilView => D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
                DescriptorType::ConstantBufferView
                | DescriptorType::ShaderResourceView
                | DescriptorType::UnorderedAccessView => D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
                DescriptorType::Sampler => D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
            };

            // 设置标志
            let flags = if desc.shader_visible {
                D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE
            } else {
                D3D12_DESCRIPTOR_HEAP_FLAG_NONE
            };

            // 创建描述符堆
            let heap_desc = D3D12_DESCRIPTOR_HEAP_DESC {
                Type: heap_type,
                NumDescriptors: desc.num_descriptors,
                Flags: flags,
                NodeMask: 0,
            };

            let heap: ID3D12DescriptorHeap = device
                .CreateDescriptorHeap(&heap_desc)
                .map_err(|e| {
                    DistRenderError::Graphics(GraphicsError::ResourceCreation(format!(
                        "Failed to create {} descriptor heap: {:?}",
                        desc.descriptor_type.name(),
                        e
                    )))
                })?;

            // 设置调试名称
            if let Some(name) = &desc.name {
                let wide_name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
                let _ = heap.SetName(windows::core::PCWSTR(wide_name.as_ptr()));
            }

            // 获取增量大小
            let increment_size = device.GetDescriptorHandleIncrementSize(heap_type);

            // 获取CPU句柄基址
            let cpu_handle = heap.GetCPUDescriptorHandleForHeapStart();
            let cpu_start = cpu_handle.ptr;

            // 如果是着色器可见，获取GPU句柄基址
            let gpu_start = if desc.shader_visible {
                let gpu_handle = heap.GetGPUDescriptorHandleForHeapStart();
                Some(gpu_handle.ptr)
            } else {
                None
            };

            Ok(Self {
                heap,
                descriptor_type: desc.descriptor_type,
                increment_size,
                cpu_start,
                gpu_start,
                num_descriptors: desc.num_descriptors,
                shader_visible: desc.shader_visible,
            })
        }
    }

    /// 获取底层 DX12 描述符堆
    pub fn heap(&self) -> &ID3D12DescriptorHeap {
        &self.heap
    }

    /// 获取描述符类型
    pub fn descriptor_type(&self) -> DescriptorType {
        self.descriptor_type
    }

    /// 获取描述符增量大小
    pub fn increment_size(&self) -> u32 {
        self.increment_size
    }

    /// 获取描述符数量
    pub fn num_descriptors(&self) -> u32 {
        self.num_descriptors
    }

    /// 是否着色器可见
    pub fn is_shader_visible(&self) -> bool {
        self.shader_visible
    }

    /// 获取 CPU 句柄基址
    pub fn cpu_start(&self) -> usize {
        self.cpu_start
    }

    /// 获取 GPU 句柄基址
    pub fn gpu_start(&self) -> Option<u64> {
        self.gpu_start
    }

    /// 获取指定索引的 CPU 句柄
    pub fn cpu_handle(&self, index: u32) -> CpuDescriptorHandle {
        CpuDescriptorHandle::new(
            self.cpu_start + (index * self.increment_size) as usize,
            index,
        )
    }

    /// 获取指定索引的 GPU 句柄（仅对着色器可见的堆）
    pub fn gpu_handle(&self, index: u32) -> Option<GpuDescriptorHandle> {
        self.gpu_start
            .map(|start| GpuDescriptorHandle::new(start + (index * self.increment_size) as u64, index))
    }

    /// 转换为 DX12 CPU 描述符句柄
    pub fn to_dx12_cpu_handle(&self, handle: CpuDescriptorHandle) -> D3D12_CPU_DESCRIPTOR_HANDLE {
        D3D12_CPU_DESCRIPTOR_HANDLE { ptr: handle.ptr }
    }

    /// 转换为 DX12 GPU 描述符句柄
    pub fn to_dx12_gpu_handle(&self, handle: GpuDescriptorHandle) -> D3D12_GPU_DESCRIPTOR_HANDLE {
        D3D12_GPU_DESCRIPTOR_HANDLE { ptr: handle.ptr }
    }
}

// DX12 堆是线程安全的
unsafe impl Send for Dx12DescriptorHeap {}
unsafe impl Sync for Dx12DescriptorHeap {}

/// DX12 描述符管理器
///
/// 扩展基础描述符管理器，添加 DX12 特定功能。
pub struct Dx12DescriptorManager {
    /// 基础描述符管理器
    base: DescriptorManager,
    /// RTV 堆
    rtv_heap: Option<Arc<Dx12DescriptorHeap>>,
    /// DSV 堆
    dsv_heap: Option<Arc<Dx12DescriptorHeap>>,
    /// SRV/CBV/UAV 堆
    srv_cbv_uav_heap: Option<Arc<Dx12DescriptorHeap>>,
    /// 采样器堆
    sampler_heap: Option<Arc<Dx12DescriptorHeap>>,
}

impl Dx12DescriptorManager {
    /// 创建新的 DX12 描述符管理器
    pub fn new() -> Self {
        Self {
            base: DescriptorManager::new(),
            rtv_heap: None,
            dsv_heap: None,
            srv_cbv_uav_heap: None,
            sampler_heap: None,
        }
    }

    /// 初始化 RTV 堆
    pub fn init_rtv_heap(&mut self, device: &ID3D12Device, num_descriptors: u32) -> Result<()> {
        let desc = DescriptorHeapDescriptor::rtv(num_descriptors);
        let heap = Dx12DescriptorHeap::new(device, &desc)?;
        let increment_size = heap.increment_size();

        self.base.init_rtv(num_descriptors, increment_size);
        self.rtv_heap = Some(Arc::new(heap));

        Ok(())
    }

    /// 初始化 DSV 堆
    pub fn init_dsv_heap(&mut self, device: &ID3D12Device, num_descriptors: u32) -> Result<()> {
        let desc = DescriptorHeapDescriptor::dsv(num_descriptors);
        let heap = Dx12DescriptorHeap::new(device, &desc)?;
        let increment_size = heap.increment_size();

        self.base.init_dsv(num_descriptors, increment_size);
        self.dsv_heap = Some(Arc::new(heap));

        Ok(())
    }

    /// 初始化 SRV/CBV/UAV 堆
    pub fn init_srv_cbv_uav_heap(&mut self, device: &ID3D12Device, num_descriptors: u32) -> Result<()> {
        let desc = DescriptorHeapDescriptor::srv_cbv_uav(num_descriptors);
        let heap = Dx12DescriptorHeap::new(device, &desc)?;
        let increment_size = heap.increment_size();

        self.base
            .init_srv_cbv_uav(num_descriptors, increment_size);
        self.srv_cbv_uav_heap = Some(Arc::new(heap));

        Ok(())
    }

    /// 初始化采样器堆
    pub fn init_sampler_heap(&mut self, device: &ID3D12Device, num_descriptors: u32) -> Result<()> {
        let desc = DescriptorHeapDescriptor::sampler(num_descriptors);
        let heap = Dx12DescriptorHeap::new(device, &desc)?;
        let increment_size = heap.increment_size();

        self.base.init_sampler(num_descriptors, increment_size);
        self.sampler_heap = Some(Arc::new(heap));

        Ok(())
    }

    /// 获取基础描述符管理器
    pub fn base(&self) -> &DescriptorManager {
        &self.base
    }

    /// 获取基础描述符管理器（可变）
    pub fn base_mut(&mut self) -> &mut DescriptorManager {
        &mut self.base
    }

    /// 获取 RTV 堆
    pub fn rtv_heap(&self) -> Option<&Arc<Dx12DescriptorHeap>> {
        self.rtv_heap.as_ref()
    }

    /// 获取 DSV 堆
    pub fn dsv_heap(&self) -> Option<&Arc<Dx12DescriptorHeap>> {
        self.dsv_heap.as_ref()
    }

    /// 获取 SRV/CBV/UAV 堆
    pub fn srv_cbv_uav_heap(&self) -> Option<&Arc<Dx12DescriptorHeap>> {
        self.srv_cbv_uav_heap.as_ref()
    }

    /// 获取采样器堆
    pub fn sampler_heap(&self) -> Option<&Arc<Dx12DescriptorHeap>> {
        self.sampler_heap.as_ref()
    }

    /// 获取着色器可见的堆数组（用于 SetDescriptorHeaps）
    ///
    /// 返回需要绑定到命令列表的堆数组
    pub fn shader_visible_heaps(&self) -> Vec<Option<ID3D12DescriptorHeap>> {
        let mut heaps = Vec::new();

        if let Some(heap) = &self.srv_cbv_uav_heap {
            if heap.is_shader_visible() {
                heaps.push(Some(heap.heap().clone()));
            }
        }

        if let Some(heap) = &self.sampler_heap {
            if heap.is_shader_visible() {
                heaps.push(Some(heap.heap().clone()));
            }
        }

        heaps
    }
}

impl Default for Dx12DescriptorManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：这些测试需要真实的 DX12 设备才能运行
    // 在 CI 环境中可能无法运行

    #[test]
    fn test_descriptor_type_conversion() {
        // 测试描述符类型名称
        assert_eq!(DescriptorType::RenderTargetView.name(), "RTV");
        assert_eq!(DescriptorType::DepthStencilView.name(), "DSV");
        assert_eq!(DescriptorType::ShaderResourceView.name(), "SRV");
    }

    #[test]
    fn test_descriptor_heap_descriptor() {
        let desc = DescriptorHeapDescriptor::rtv(100);
        assert_eq!(desc.num_descriptors, 100);
        assert_eq!(desc.descriptor_type, DescriptorType::RenderTargetView);
        assert!(!desc.shader_visible);

        let desc = DescriptorHeapDescriptor::srv_cbv_uav(128);
        assert!(desc.shader_visible);
    }

    #[test]
    fn test_dx12_descriptor_manager_creation() {
        let manager = Dx12DescriptorManager::new();
        assert!(manager.rtv_heap().is_none());
        assert!(manager.dsv_heap().is_none());
        assert!(manager.srv_cbv_uav_heap().is_none());
        assert!(manager.sampler_heap().is_none());
    }
}
