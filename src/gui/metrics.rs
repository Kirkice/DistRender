//! 性能统计模块
//!
//! PerformanceMetrics 用于跟踪和计算帧率、帧时间等性能指标。

use std::time::{Duration, Instant};

/// 性能统计（帧率、帧时间）
pub struct PerformanceMetrics {
    frame_count: u32,
    last_update: Instant,
    fps: f32,
    frame_time_ms: f32,
    frame_times: Vec<f32>,  // 用于平滑计算（预留）
}

impl PerformanceMetrics {
    /// 创建新的性能统计器
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            last_update: Instant::now(),
            fps: 0.0,
            frame_time_ms: 0.0,
            frame_times: Vec::with_capacity(60),
        }
    }

    /// 记录一帧
    pub fn record_frame(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);

        // 每秒更新一次 FPS
        if elapsed >= Duration::from_secs(1) {
            self.fps = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_time_ms = 1000.0 / self.fps;
            self.frame_count = 0;
            self.last_update = now;
        }
    }

    /// 获取当前 FPS
    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// 获取当前帧时间（毫秒）
    pub fn frame_time_ms(&self) -> f32 {
        self.frame_time_ms
    }
}
