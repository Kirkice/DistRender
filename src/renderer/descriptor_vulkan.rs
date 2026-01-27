//! Vulkan 描述符管理实现
//!
//! 提供 Vulkan 特定的描述符集（Descriptor Set）管理功能。
//! Vulkan 使用描述符池（Descriptor Pool）和描述符集的概念。

use crate::core::error::{Result, DistRenderError, GraphicsError};
use crate::renderer::descriptor::DescriptorType;
use std::sync::Arc;
use vulkano::descriptor_set::allocator::{StandardDescriptorSetAllocator};
use vulkano::descriptor_set::layout::{DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType as VkDescriptorType};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::Device;
use vulkano::shader::ShaderStages;

/// Vulkan 描述符类型映射
fn to_vulkan_descriptor_type(desc_type: DescriptorType) -> VkDescriptorType {
    match desc_type {
        DescriptorType::ConstantBufferView => VkDescriptorType::UniformBuffer,
        DescriptorType::ShaderResourceView => VkDescriptorType::SampledImage,
        DescriptorType::UnorderedAccessView => VkDescriptorType::StorageImage,
        DescriptorType::Sampler => VkDescriptorType::Sampler,
        // Vulkan 没有直接对应 RTV/DSV 的描述符类型
        // 这些通过 RenderPass 和 Framebuffer 管理
        DescriptorType::RenderTargetView | DescriptorType::DepthStencilView => {
            VkDescriptorType::InputAttachment
        }
    }
}

/// Vulkan 描述符集布局绑定信息
#[derive(Debug, Clone)]
pub struct VulkanDescriptorBinding {
    /// 绑定点
    pub binding: u32,
    /// 描述符类型
    pub descriptor_type: DescriptorType,
    /// 描述符数量
    pub descriptor_count: u32,
    /// 着色器阶段
    pub shader_stages: ShaderStages,
}

impl VulkanDescriptorBinding {
    /// 创建新的绑定信息
    pub fn new(
        binding: u32,
        descriptor_type: DescriptorType,
        descriptor_count: u32,
        shader_stages: ShaderStages,
    ) -> Self {
        Self {
            binding,
            descriptor_type,
            descriptor_count,
            shader_stages,
        }
    }

    /// 创建 Uniform Buffer 绑定
    pub fn uniform_buffer(binding: u32, stages: ShaderStages) -> Self {
        Self::new(binding, DescriptorType::ConstantBufferView, 1, stages)
    }

    /// 创建 Storage Buffer 绑定
    pub fn storage_buffer(binding: u32, stages: ShaderStages) -> Self {
        Self::new(binding, DescriptorType::UnorderedAccessView, 1, stages)
    }

    /// 创建 Sampled Image 绑定
    pub fn sampled_image(binding: u32, stages: ShaderStages) -> Self {
        Self::new(binding, DescriptorType::ShaderResourceView, 1, stages)
    }

    /// 创建 Sampler 绑定
    pub fn sampler(binding: u32, stages: ShaderStages) -> Self {
        Self::new(binding, DescriptorType::Sampler, 1, stages)
    }

    /// 转换为 Vulkano 的绑定描述
    pub fn to_vulkano_binding(&self) -> DescriptorSetLayoutBinding {
        DescriptorSetLayoutBinding {
            stages: self.shader_stages,
            ..DescriptorSetLayoutBinding::descriptor_type(
                to_vulkan_descriptor_type(self.descriptor_type)
            )
        }
    }
}

/// Vulkan 描述符集布局
///
/// 定义描述符集的结构和绑定点。
pub struct VulkanDescriptorSetLayout {
    /// Vulkano 描述符集布局
    layout: Arc<DescriptorSetLayout>,
    /// 绑定信息
    bindings: Vec<VulkanDescriptorBinding>,
}

impl VulkanDescriptorSetLayout {
    /// 创建新的描述符集布局
    ///
    /// # 参数
    ///
    /// * `device` - Vulkan 设备
    /// * `bindings` - 绑定信息列表
    ///
    /// # 返回值
    ///
    /// 返回新创建的描述符集布局
    pub fn new(device: Arc<Device>, bindings: Vec<VulkanDescriptorBinding>) -> Result<Self> {
        // 转换为 Vulkano 绑定
        let vk_bindings: Vec<_> = bindings
            .iter()
            .map(|b| (b.binding, b.to_vulkano_binding()))
            .collect();

        // 创建布局
        let create_info = DescriptorSetLayoutCreateInfo {
            bindings: vk_bindings.into_iter().collect(),
            ..Default::default()
        };

        let layout = DescriptorSetLayout::new(device.clone(), create_info).map_err(|e| {
            DistRenderError::Graphics(GraphicsError::ResourceCreation(format!(
                "Failed to create descriptor set layout: {:?}",
                e
            )))
        })?;

        Ok(Self { layout, bindings })
    }

    /// 获取 Vulkano 描述符集布局
    pub fn layout(&self) -> &Arc<DescriptorSetLayout> {
        &self.layout
    }

    /// 获取绑定信息
    pub fn bindings(&self) -> &[VulkanDescriptorBinding] {
        &self.bindings
    }
}

/// Vulkan 描述符池大小
#[derive(Debug, Clone)]
pub struct VulkanDescriptorPoolSize {
    /// 描述符类型
    pub descriptor_type: DescriptorType,
    /// 描述符数量
    pub descriptor_count: u32,
}

impl VulkanDescriptorPoolSize {
    /// 创建新的池大小描述
    pub fn new(descriptor_type: DescriptorType, descriptor_count: u32) -> Self {
        Self {
            descriptor_type,
            descriptor_count,
        }
    }
}

/// Vulkan 描述符管理器
///
/// 管理 Vulkan 描述符集的分配和生命周期。
/// 使用 Vulkano 的 StandardDescriptorSetAllocator 进行内存管理。
pub struct VulkanDescriptorManager {
    /// 设备引用
    device: Arc<Device>,
    /// 描述符集分配器
    allocator: StandardDescriptorSetAllocator,
    /// 描述符集布局缓存
    layouts: Vec<Arc<VulkanDescriptorSetLayout>>,
}

impl VulkanDescriptorManager {
    /// 创建新的 Vulkan 描述符管理器
    ///
    /// # 参数
    ///
    /// * `device` - Vulkan 设备
    ///
    /// # 返回值
    ///
    /// 返回新创建的描述符管理器
    pub fn new(device: Arc<Device>) -> Self {
        let allocator = StandardDescriptorSetAllocator::new(device.clone());

        Self {
            device,
            allocator,
            layouts: Vec::new(),
        }
    }

    /// 获取设备引用
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    /// 获取描述符集分配器
    pub fn allocator(&self) -> &StandardDescriptorSetAllocator {
        &self.allocator
    }

    /// 创建并缓存描述符集布局
    ///
    /// # 参数
    ///
    /// * `bindings` - 绑定信息列表
    ///
    /// # 返回值
    ///
    /// 返回布局的索引
    pub fn create_layout(&mut self, bindings: Vec<VulkanDescriptorBinding>) -> Result<usize> {
        let layout = VulkanDescriptorSetLayout::new(self.device.clone(), bindings)?;
        let index = self.layouts.len();
        self.layouts.push(Arc::new(layout));
        Ok(index)
    }

    /// 获取描述符集布局
    pub fn get_layout(&self, index: usize) -> Option<&Arc<VulkanDescriptorSetLayout>> {
        self.layouts.get(index)
    }

    /// 分配描述符集
    ///
    /// # 参数
    ///
    /// * `layout_index` - 布局索引
    /// * `writes` - 描述符写入操作
    ///
    /// # 返回值
    ///
    /// 返回分配的描述符集
    pub fn allocate_descriptor_set(
        &self,
        layout_index: usize,
        writes: impl IntoIterator<Item = WriteDescriptorSet>,
    ) -> Result<Arc<PersistentDescriptorSet>> {
        let layout = self
            .get_layout(layout_index)
            .ok_or_else(|| {
                DistRenderError::Runtime(format!(
                    "Descriptor set layout with index {} not found",
                    layout_index
                ))
            })?;

        let descriptor_set = PersistentDescriptorSet::new(
            &self.allocator,
            layout.layout().clone(),
            writes,
        )
        .map_err(|e| {
            DistRenderError::Graphics(GraphicsError::ResourceCreation(format!(
                "Failed to allocate descriptor set: {:?}",
                e
            )))
        })?;

        Ok(descriptor_set)
    }

    /// 获取布局数量
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }
}

/// 描述符集构建器
///
/// 用于构建描述符集的辅助工具。
pub struct DescriptorSetBuilder {
    bindings: Vec<VulkanDescriptorBinding>,
}

impl DescriptorSetBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    /// 添加 Uniform Buffer 绑定
    pub fn add_uniform_buffer(mut self, binding: u32, stages: ShaderStages) -> Self {
        self.bindings
            .push(VulkanDescriptorBinding::uniform_buffer(binding, stages));
        self
    }

    /// 添加 Storage Buffer 绑定
    pub fn add_storage_buffer(mut self, binding: u32, stages: ShaderStages) -> Self {
        self.bindings
            .push(VulkanDescriptorBinding::storage_buffer(binding, stages));
        self
    }

    /// 添加 Sampled Image 绑定
    pub fn add_sampled_image(mut self, binding: u32, stages: ShaderStages) -> Self {
        self.bindings
            .push(VulkanDescriptorBinding::sampled_image(binding, stages));
        self
    }

    /// 添加 Sampler 绑定
    pub fn add_sampler(mut self, binding: u32, stages: ShaderStages) -> Self {
        self.bindings
            .push(VulkanDescriptorBinding::sampler(binding, stages));
        self
    }

    /// 添加自定义绑定
    pub fn add_binding(mut self, binding: VulkanDescriptorBinding) -> Self {
        self.bindings.push(binding);
        self
    }

    /// 构建描述符集布局
    pub fn build(self, device: Arc<Device>) -> Result<VulkanDescriptorSetLayout> {
        VulkanDescriptorSetLayout::new(device, self.bindings)
    }
}

impl Default for DescriptorSetBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descriptor_type_mapping() {
        let vk_type = to_vulkan_descriptor_type(DescriptorType::ConstantBufferView);
        assert_eq!(vk_type, VkDescriptorType::UniformBuffer);

        let vk_type = to_vulkan_descriptor_type(DescriptorType::ShaderResourceView);
        assert_eq!(vk_type, VkDescriptorType::SampledImage);
    }

    #[test]
    fn test_descriptor_binding() {
        // Use empty shader stages for testing structure
        let stages = ShaderStages::empty();
        let binding = VulkanDescriptorBinding::uniform_buffer(0, stages);
        assert_eq!(binding.binding, 0);
        assert_eq!(binding.descriptor_type, DescriptorType::ConstantBufferView);
        assert_eq!(binding.descriptor_count, 1);
    }

    #[test]
    fn test_descriptor_set_builder() {
        // Use empty shader stages for testing structure
        let stages = ShaderStages::empty();
        let builder = DescriptorSetBuilder::new()
            .add_uniform_buffer(0, stages)
            .add_sampled_image(1, stages)
            .add_sampler(2, stages);

        assert_eq!(builder.bindings.len(), 3);
        assert_eq!(builder.bindings[0].binding, 0);
        assert_eq!(builder.bindings[1].binding, 1);
        assert_eq!(builder.bindings[2].binding, 2);
    }
}
