//! 命令缓冲区管理模块
//!
//! 提供统一的命令缓冲区管理接口，封装不同图形API的命令记录机制。
//! 借鉴 DistEngine 的 CommandList 设计。
//!
//! # 设计原则
//!
//! - **统一接口**：为Vulkan和DirectX提供统一的命令记录抽象
//! - **资源管理**：自动管理命令缓冲区的分配和释放
//! - **类型安全**：通过类型系统防止错误的命令提交
//! - **性能优化**：支持命令缓冲区复用和批处理
//!
//! # 命令缓冲区类型
//!
//! - **Direct**: 图形和计算命令
//! - **Bundle**: 可复用的命令包（DX12）/ Secondary（Vulkan）
//! - **Compute**: 计算专用命令
//! - **Transfer**: 传输专用命令

use crate::core::error::{Result, DistRenderError, GraphicsError};

/// 命令缓冲区类型
///
/// 对应 DistEngine 的 CommandListType
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferType {
    /// 直接命令缓冲区（Primary）
    /// 可以包含所有类型的命令
    Direct,

    /// 间接命令缓冲区（Secondary/Bundle）
    /// 可以被Direct命令缓冲区调用，用于复用
    Bundle,

    /// 计算专用命令缓冲区
    /// 只能包含计算命令
    Compute,

    /// 传输专用命令缓冲区
    /// 只能包含传输命令（复制、清空等）
    Transfer,
}

/// 命令缓冲区状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferState {
    /// 初始状态
    Initial,
    /// 正在记录
    Recording,
    /// 已完成记录
    Executable,
    /// 正在执行
    Pending,
    /// 无效状态
    Invalid,
}

/// 命令缓冲区使用模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandBufferUsage {
    /// 一次性使用
    /// 提交后自动重置
    OneTimeSubmit,

    /// 可重复使用
    /// 需要手动重置
    Reusable,

    /// 同步重新录制
    /// Vulkan特有
    SimultaneousUse,
}

/// 命令缓冲区描述符
#[derive(Debug, Clone)]
pub struct CommandBufferDescriptor {
    /// 缓冲区类型
    pub buffer_type: CommandBufferType,
    /// 使用模式
    pub usage: CommandBufferUsage,
    /// 调试名称
    pub name: Option<String>,
}

impl CommandBufferDescriptor {
    /// 创建新的命令缓冲区描述符
    pub fn new(buffer_type: CommandBufferType, usage: CommandBufferUsage) -> Self {
        Self {
            buffer_type,
            usage,
            name: None,
        }
    }

    /// 设置调试名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 创建Direct类型的一次性命令缓冲区
    pub fn direct_one_time() -> Self {
        Self::new(CommandBufferType::Direct, CommandBufferUsage::OneTimeSubmit)
    }

    /// 创建Direct类型的可复用命令缓冲区
    pub fn direct_reusable() -> Self {
        Self::new(CommandBufferType::Direct, CommandBufferUsage::Reusable)
    }

    /// 创建Transfer类型的命令缓冲区
    pub fn transfer_one_time() -> Self {
        Self::new(CommandBufferType::Transfer, CommandBufferUsage::OneTimeSubmit)
    }
}

/// 命令缓冲区池
///
/// 管理命令缓冲区的分配和回收，类似于 CommandAllocator（DX12）或 CommandPool（Vulkan）
#[derive(Debug)]
pub struct CommandBufferPool {
    /// 命令缓冲区类型
    buffer_type: CommandBufferType,
    /// 池中的缓冲区数量
    capacity: usize,
    /// 当前已分配的数量
    allocated: usize,
}

impl CommandBufferPool {
    /// 创建新的命令缓冲区池
    ///
    /// # 参数
    ///
    /// * `buffer_type` - 缓冲区类型
    /// * `capacity` - 池容量（预分配数量）
    pub fn new(buffer_type: CommandBufferType, capacity: usize) -> Self {
        Self {
            buffer_type,
            capacity,
            allocated: 0,
        }
    }

    /// 获取缓冲区类型
    pub fn buffer_type(&self) -> CommandBufferType {
        self.buffer_type
    }

    /// 获取池容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 获取已分配数量
    pub fn allocated(&self) -> usize {
        self.allocated
    }

    /// 是否已满
    pub fn is_full(&self) -> bool {
        self.allocated >= self.capacity
    }

    /// 标记分配一个缓冲区
    pub fn allocate(&mut self) -> Result<()> {
        if self.is_full() {
            return Err(DistRenderError::Graphics(
                GraphicsError::ResourceCreation("Command buffer pool is full".to_string())
            ));
        }
        self.allocated += 1;
        Ok(())
    }

    /// 释放一个缓冲区
    pub fn free(&mut self) {
        if self.allocated > 0 {
            self.allocated -= 1;
        }
    }

    /// 重置池（释放所有缓冲区）
    pub fn reset(&mut self) {
        self.allocated = 0;
    }
}

/// 命令编码器
///
/// 提供类型安全的命令记录接口
/// 使用建造者模式确保命令记录的正确性
pub struct CommandEncoder {
    /// 缓冲区类型
    buffer_type: CommandBufferType,
    /// 当前状态
    state: CommandBufferState,
    /// 是否在渲染通道中
    in_render_pass: bool,
}

impl CommandEncoder {
    /// 创建新的命令编码器
    pub fn new(buffer_type: CommandBufferType) -> Self {
        Self {
            buffer_type,
            state: CommandBufferState::Initial,
            in_render_pass: false,
        }
    }

    /// 开始记录命令
    pub fn begin(&mut self) -> Result<()> {
        match self.state {
            CommandBufferState::Initial | CommandBufferState::Executable => {
                self.state = CommandBufferState::Recording;
                Ok(())
            }
            _ => Err(DistRenderError::Graphics(
                GraphicsError::CommandExecution("Invalid state for begin".to_string())
            )),
        }
    }

    /// 结束记录命令
    pub fn end(&mut self) -> Result<()> {
        match self.state {
            CommandBufferState::Recording => {
                if self.in_render_pass {
                    return Err(DistRenderError::Graphics(
                        GraphicsError::CommandExecution("Still in render pass".to_string())
                    ));
                }
                self.state = CommandBufferState::Executable;
                Ok(())
            }
            _ => Err(DistRenderError::Graphics(
                GraphicsError::CommandExecution("Invalid state for end".to_string())
            )),
        }
    }

    /// 开始渲染通道
    pub fn begin_render_pass(&mut self) -> Result<()> {
        if self.buffer_type != CommandBufferType::Direct {
            return Err(DistRenderError::Graphics(
                GraphicsError::CommandExecution("Only Direct buffers can begin render pass".to_string())
            ));
        }

        if self.state != CommandBufferState::Recording {
            return Err(DistRenderError::Graphics(
                GraphicsError::CommandExecution("Must be in recording state".to_string())
            ));
        }

        if self.in_render_pass {
            return Err(DistRenderError::Graphics(
                GraphicsError::CommandExecution("Already in render pass".to_string())
            ));
        }

        self.in_render_pass = true;
        Ok(())
    }

    /// 结束渲染通道
    pub fn end_render_pass(&mut self) -> Result<()> {
        if !self.in_render_pass {
            return Err(DistRenderError::Graphics(
                GraphicsError::CommandExecution("Not in render pass".to_string())
            ));
        }

        self.in_render_pass = false;
        Ok(())
    }

    /// 重置编码器
    pub fn reset(&mut self) {
        self.state = CommandBufferState::Initial;
        self.in_render_pass = false;
    }

    /// 获取当前状态
    pub fn state(&self) -> CommandBufferState {
        self.state
    }

    /// 是否在渲染通道中
    pub fn is_in_render_pass(&self) -> bool {
        self.in_render_pass
    }
}

/// 命令队列类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueType {
    /// 图形队列（支持图形、计算、传输）
    Graphics,
    /// 计算队列（支持计算、传输）
    Compute,
    /// 传输队列（仅支持传输）
    Transfer,
}

/// 命令提交信息
#[derive(Debug)]
pub struct SubmitInfo {
    /// 命令缓冲区数量
    pub command_buffer_count: usize,
    /// 等待信号量数量
    pub wait_semaphore_count: usize,
    /// Signal信号量数量
    pub signal_semaphore_count: usize,
}

impl SubmitInfo {
    /// 创建简单的提交信息（单个命令缓冲区，无同步）
    pub fn simple() -> Self {
        Self {
            command_buffer_count: 1,
            wait_semaphore_count: 0,
            signal_semaphore_count: 0,
        }
    }

    /// 创建带同步的提交信息
    pub fn with_sync(wait_count: usize, signal_count: usize) -> Self {
        Self {
            command_buffer_count: 1,
            wait_semaphore_count: wait_count,
            signal_semaphore_count: signal_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_buffer_descriptor() {
        let desc = CommandBufferDescriptor::direct_one_time();
        assert_eq!(desc.buffer_type, CommandBufferType::Direct);
        assert_eq!(desc.usage, CommandBufferUsage::OneTimeSubmit);

        let desc2 = CommandBufferDescriptor::transfer_one_time()
            .with_name("Upload Buffer");
        assert_eq!(desc2.buffer_type, CommandBufferType::Transfer);
        assert_eq!(desc2.name.as_deref(), Some("Upload Buffer"));
    }

    #[test]
    fn test_command_buffer_pool() {
        let mut pool = CommandBufferPool::new(CommandBufferType::Direct, 5);

        assert_eq!(pool.capacity(), 5);
        assert_eq!(pool.allocated(), 0);
        assert!(!pool.is_full());

        for _ in 0..5 {
            pool.allocate().unwrap();
        }

        assert!(pool.is_full());
        assert_eq!(pool.allocated(), 5);

        // 第6次分配应该失败
        assert!(pool.allocate().is_err());

        pool.reset();
        assert_eq!(pool.allocated(), 0);
    }

    #[test]
    fn test_command_encoder_state_machine() {
        let mut encoder = CommandEncoder::new(CommandBufferType::Direct);

        assert_eq!(encoder.state(), CommandBufferState::Initial);

        // 开始记录
        encoder.begin().unwrap();
        assert_eq!(encoder.state(), CommandBufferState::Recording);

        // 不能重复开始
        assert!(encoder.begin().is_err());

        // 开始渲染通道
        encoder.begin_render_pass().unwrap();
        assert!(encoder.is_in_render_pass());

        // 在渲染通道中不能结束命令缓冲区
        assert!(encoder.end().is_err());

        // 结束渲染通道
        encoder.end_render_pass().unwrap();
        assert!(!encoder.is_in_render_pass());

        // 现在可以结束
        encoder.end().unwrap();
        assert_eq!(encoder.state(), CommandBufferState::Executable);
    }

    #[test]
    fn test_command_encoder_type_restrictions() {
        let mut encoder = CommandEncoder::new(CommandBufferType::Compute);

        encoder.begin().unwrap();

        // 计算命令缓冲区不能开始渲染通道
        assert!(encoder.begin_render_pass().is_err());
    }

    #[test]
    fn test_submit_info() {
        let info = SubmitInfo::simple();
        assert_eq!(info.command_buffer_count, 1);
        assert_eq!(info.wait_semaphore_count, 0);
        assert_eq!(info.signal_semaphore_count, 0);

        let info2 = SubmitInfo::with_sync(2, 1);
        assert_eq!(info2.wait_semaphore_count, 2);
        assert_eq!(info2.signal_semaphore_count, 1);
    }
}
