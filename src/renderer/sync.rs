//! GPU 同步机制模块
//!
//! 提供统一的GPU同步原语，用于CPU-GPU同步和GPU-GPU同步。
//! 借鉴 DistEngine 的 Fence 同步机制设计。
//!
//! # 设计原则
//!
//! - **Fence同步**：用于CPU等待GPU完成工作
//! - **Semaphore同步**：用于GPU命令之间的同步
//! - **统一接口**：为不同图形API提供统一的同步抽象
//!
//! # 使用场景
//!
//! 1. **帧同步**：确保GPU完成前一帧才开始下一帧
//! 2. **资源更新**：确保资源在使用前已更新完成
//! 3. **多队列协作**：图形队列和计算队列之间的同步

use crate::core::error::{Result, DistRenderError, GraphicsError};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Fence 值
///
/// 用于CPU-GPU同步的单调递增值。
/// CPU可以等待GPU完成特定Fence值对应的工作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FenceValue(u64);

impl FenceValue {
    /// 创建新的Fence值
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// 获取内部值
    pub fn value(&self) -> u64 {
        self.0
    }

    /// 递增Fence值
    pub fn increment(&mut self) {
        self.0 += 1;
    }

    /// 下一个Fence值
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }
}

/// Fence 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceStatus {
    /// 尚未signal
    NotSignaled,
    /// 已经signal
    Signaled,
    /// 错误状态
    Error,
}

/// Fence 管理器
///
/// 管理多个Fence值，用于跟踪GPU工作进度。
/// 类似于 DistEngine 的 FlushCommandQueue 机制。
///
/// # 示例
///
/// ```rust
/// let mut fence_manager = FenceManager::new();
///
/// // 提交工作并signal
/// let fence_value = fence_manager.next_value();
/// command_queue.signal(fence, fence_value);
///
/// // 等待完成
/// fence_manager.wait_for_value(fence_value)?;
/// ```
pub struct FenceManager {
    /// 当前Fence值（CPU侧）
    current_value: Arc<AtomicU64>,
    /// 已完成的Fence值（GPU侧）
    completed_value: Arc<AtomicU64>,
}

impl FenceManager {
    /// 创建新的Fence管理器
    pub fn new() -> Self {
        Self {
            current_value: Arc::new(AtomicU64::new(0)),
            completed_value: Arc::new(AtomicU64::new(0)),
        }
    }

    /// 获取当前Fence值
    pub fn current_value(&self) -> FenceValue {
        FenceValue::new(self.current_value.load(Ordering::Acquire))
    }

    /// 获取已完成的Fence值
    pub fn completed_value(&self) -> FenceValue {
        FenceValue::new(self.completed_value.load(Ordering::Acquire))
    }

    /// 获取下一个Fence值并递增计数器
    pub fn next_value(&self) -> FenceValue {
        let value = self.current_value.fetch_add(1, Ordering::AcqRel);
        FenceValue::new(value + 1)
    }

    /// 更新已完成的Fence值
    ///
    /// 通常在GPU完成工作后由驱动调用
    pub fn update_completed_value(&self, value: FenceValue) {
        self.completed_value.store(value.value(), Ordering::Release);
    }

    /// 检查特定Fence值是否已完成
    pub fn is_completed(&self, value: FenceValue) -> bool {
        self.completed_value() >= value
    }

    /// 等待特定Fence值完成
    ///
    /// 这是一个阻塞操作，会等待直到GPU完成工作
    pub fn wait_for_value(&self, value: FenceValue) -> Result<()> {
        // 注意：实际实现需要调用图形API的等待函数
        // 这里只是接口定义
        while !self.is_completed(value) {
            std::thread::yield_now();
        }
        Ok(())
    }

    /// 刷新命令队列（等待所有工作完成）
    ///
    /// 类似于 DistEngine 的 FlushCommandQueue
    pub fn flush(&self) -> Result<()> {
        let current = self.current_value();
        self.wait_for_value(current)
    }

    /// 重置Fence管理器
    pub fn reset(&self) {
        self.current_value.store(0, Ordering::Release);
        self.completed_value.store(0, Ordering::Release);
    }
}

impl Default for FenceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// GPU 时间线
///
/// 跟踪GPU工作的时间线，用于性能分析和调试。
#[derive(Debug)]
pub struct Timeline {
    /// 帧开始时间
    frame_start: FenceValue,
    /// 帧结束时间
    frame_end: FenceValue,
    /// 帧号
    frame_number: u64,
}

impl Timeline {
    /// 创建新的时间线
    pub fn new(frame_number: u64, start: FenceValue) -> Self {
        Self {
            frame_start: start,
            frame_end: start,
            frame_number,
        }
    }

    /// 标记帧结束
    pub fn end_frame(&mut self, end: FenceValue) {
        self.frame_end = end;
    }

    /// 获取帧持续时间（Fence值差）
    pub fn duration(&self) -> u64 {
        self.frame_end.value().saturating_sub(self.frame_start.value())
    }

    /// 获取帧号
    pub fn frame_number(&self) -> u64 {
        self.frame_number
    }
}

/// Semaphore 类型
///
/// 用于GPU命令之间的同步（不涉及CPU）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreType {
    /// 二进制Semaphore（传统类型）
    Binary,
    /// 时间线Semaphore（支持计数）
    Timeline,
}

/// GPU 工作提交信息
///
/// 描述一次GPU工作提交需要的同步信息
#[derive(Debug)]
pub struct SubmitInfo {
    /// 等待的Semaphore列表
    pub wait_semaphores: Vec<SemaphoreHandle>,
    /// Signal的Semaphore列表
    pub signal_semaphores: Vec<SemaphoreHandle>,
    /// 等待的管线阶段
    pub wait_stages: Vec<PipelineStage>,
}

/// Semaphore 句柄
///
/// 对实际Semaphore对象的引用
#[derive(Debug, Clone)]
pub struct SemaphoreHandle {
    /// Semaphore ID
    id: u64,
    /// Semaphore 类型
    semaphore_type: SemaphoreType,
}

impl SemaphoreHandle {
    /// 创建新的Semaphore句柄
    pub fn new(id: u64, semaphore_type: SemaphoreType) -> Self {
        Self { id, semaphore_type }
    }

    /// 获取ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 获取类型
    pub fn semaphore_type(&self) -> SemaphoreType {
        self.semaphore_type
    }
}

/// 管线阶段
///
/// 定义GPU管线的各个阶段，用于同步
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// 顶点着色器阶段
    VertexShader,
    /// 片段着色器阶段
    FragmentShader,
    /// 计算着色器阶段
    ComputeShader,
    /// 传输阶段
    Transfer,
    /// 颜色输出阶段
    ColorOutput,
    /// 所有图形阶段
    AllGraphics,
    /// 所有命令阶段
    AllCommands,
}

/// 同步作用域
///
/// 定义同步操作的范围
pub struct SyncScope {
    /// 等待阶段
    pub wait_stages: Vec<PipelineStage>,
    /// Signal阶段
    pub signal_stages: Vec<PipelineStage>,
}

impl SyncScope {
    /// 创建全局同步作用域
    pub fn all_commands() -> Self {
        Self {
            wait_stages: vec![PipelineStage::AllCommands],
            signal_stages: vec![PipelineStage::AllCommands],
        }
    }

    /// 创建图形管线同步作用域
    pub fn graphics() -> Self {
        Self {
            wait_stages: vec![PipelineStage::AllGraphics],
            signal_stages: vec![PipelineStage::AllGraphics],
        }
    }

    /// 创建颜色输出同步作用域
    pub fn color_output() -> Self {
        Self {
            wait_stages: vec![PipelineStage::ColorOutput],
            signal_stages: vec![PipelineStage::ColorOutput],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fence_value() {
        let mut fence = FenceValue::new(0);
        assert_eq!(fence.value(), 0);

        fence.increment();
        assert_eq!(fence.value(), 1);

        let next = fence.next();
        assert_eq!(next.value(), 2);
        assert_eq!(fence.value(), 1); // 原值不变
    }

    #[test]
    fn test_fence_manager() {
        let manager = FenceManager::new();

        assert_eq!(manager.current_value().value(), 0);
        assert_eq!(manager.completed_value().value(), 0);

        let v1 = manager.next_value();
        assert_eq!(v1.value(), 1);
        assert_eq!(manager.current_value().value(), 1);

        let v2 = manager.next_value();
        assert_eq!(v2.value(), 2);

        // 模拟GPU完成
        manager.update_completed_value(v1);
        assert!(manager.is_completed(v1));
        assert!(!manager.is_completed(v2));

        manager.update_completed_value(v2);
        assert!(manager.is_completed(v2));
    }

    #[test]
    fn test_fence_ordering() {
        let f1 = FenceValue::new(1);
        let f2 = FenceValue::new(2);
        let f3 = FenceValue::new(1);

        assert!(f1 < f2);
        assert!(f2 > f1);
        assert_eq!(f1, f3);
    }

    #[test]
    fn test_timeline() {
        let start = FenceValue::new(100);
        let end = FenceValue::new(150);

        let mut timeline = Timeline::new(42, start);
        assert_eq!(timeline.frame_number(), 42);

        timeline.end_frame(end);
        assert_eq!(timeline.duration(), 50);
    }

    #[test]
    fn test_sync_scope() {
        let all = SyncScope::all_commands();
        assert_eq!(all.wait_stages.len(), 1);
        assert_eq!(all.wait_stages[0], PipelineStage::AllCommands);

        let graphics = SyncScope::graphics();
        assert_eq!(graphics.wait_stages[0], PipelineStage::AllGraphics);

        let color = SyncScope::color_output();
        assert_eq!(color.wait_stages[0], PipelineStage::ColorOutput);
    }
}
