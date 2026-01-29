//! GUI 状态管理
//!
//! GuiState 保存所有 GUI 相关的状态数据，与具体的图形后端无关。

use crate::core::Config;
use crate::core::SceneConfig;

/// GUI 状态（与后端无关）
pub struct GuiState {
    // 性能监控
    pub show_fps: bool,
    pub fps: f32,
    pub frame_time_ms: f32,

    // 渲染设置
    pub clear_color: [f32; 4],
    pub light_intensity: f32,
    pub light_direction: [f32; 3],

    // 场景控制
    pub model_position: [f32; 3],
    pub model_rotation: [f32; 3],
    pub model_scale: [f32; 3],

    // 相机参数
    pub camera_fov: f32,
    pub camera_near: f32,
    pub camera_far: f32,

    // 后端信息
    pub current_backend: String,
    pub selected_backend: String,
    pub backend_changed: bool,
}

impl GuiState {
    /// 从配置和场景创建 GUI 状态
    pub fn new(config: &Config, scene: &SceneConfig) -> Self {
        Self {
            show_fps: true,
            fps: 0.0,
            frame_time_ms: 0.0,

            clear_color: scene.clear_color,
            light_intensity: scene.light.intensity,
            light_direction: scene.light.transform.rotation,

            model_position: scene.model.transform.position,
            model_rotation: scene.model.transform.rotation,
            model_scale: scene.model.transform.scale,

            camera_fov: scene.camera.fov,
            camera_near: scene.camera.near_clip,
            camera_far: scene.camera.far_clip,

            current_backend: config.graphics.backend.name().to_string(),
            selected_backend: config.graphics.backend.name().to_string(),
            backend_changed: false,
        }
    }

    /// 更新性能统计
    pub fn update_performance(&mut self, fps: f32, frame_time_ms: f32) {
        self.fps = fps;
        self.frame_time_ms = frame_time_ms;
    }

    /// 检查后端是否改变
    pub fn check_backend_change(&mut self) -> bool {
        if self.selected_backend != self.current_backend {
            self.backend_changed = true;
            true
        } else {
            false
        }
    }
}
