//! GUI 管理器
//!
//! GuiManager 是 GUI 系统的核心，负责集成 egui 和 wgpu，
//! 处理输入事件，更新 UI 状态，并渲染 GUI。

use egui;
use egui_wgpu::Renderer as EguiRenderer;
use egui_winit::State as EguiState;
use winit::window::Window;

use crate::gui::state::GuiState;
use crate::gui::metrics::PerformanceMetrics;
use crate::gui::panels;
use crate::core::error::Result;

/// GUI 管理器（使用 egui + wgpu）
pub struct GuiManager {
    // egui 核心组件
    context: egui::Context,
    state: EguiState,
    renderer: EguiRenderer,

    // GUI 状态和统计
    gui_state: GuiState,
    metrics: PerformanceMetrics,
}

impl GuiManager {
    /// 创建 GUI 管理器
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        window: &Window,
        gui_state: GuiState,
    ) -> Result<Self> {
        // 创建 egui context
        let context = egui::Context::default();

        // 创建 egui-winit state
        let state = EguiState::new(window);

        // 创建 egui-wgpu renderer
        let renderer = EguiRenderer::new(device, surface_format, None, 1);

        let metrics = PerformanceMetrics::new();

        Ok(Self {
            context,
            state,
            renderer,
            gui_state,
            metrics,
        })
    }

    /// 处理输入事件
    /// 返回 true 如果事件被 GUI 消费
    pub fn handle_event(&mut self, event: &winit::event::WindowEvent) -> bool {
        let response = self.state.on_event(&self.context, event);
        response.consumed
    }

    /// 更新 GUI（构建 UI）
    pub fn update(&mut self, window: &Window) {
        // 记录帧
        self.metrics.record_frame();
        self.gui_state.update_performance(
            self.metrics.fps(),
            self.metrics.frame_time_ms()
        );

        // 开始新帧
        let raw_input = self.state.take_egui_input(window);
        self.context.begin_frame(raw_input);

        // 渲染侧边栏面板
        egui::SidePanel::left("control_panel")
            .default_width(300.0)
            .show(&self.context, |ui| {
                ui.heading("DistRender 控制面板");
                ui.separator();

                // 性能面板
                panels::performance::render(ui, &self.gui_state);
                ui.separator();

                // 渲染设置面板
                panels::rendering::render(ui, &mut self.gui_state);
                ui.separator();

                // 场景控制面板
                panels::scene::render(ui, &mut self.gui_state);
                ui.separator();

                // 后端切换面板
                panels::backend::render(ui, &mut self.gui_state);
            });
    }

    /// 渲染 GUI（绘制到 wgpu）
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
    ) -> Result<()> {
        // 结束帧，获取输出
        let full_output = self.context.end_frame();

        // 处理平台输出（光标、复制粘贴等）
        self.state.handle_platform_output(window, &self.context, full_output.platform_output);

        // 更新纹理和缓冲
        let paint_jobs = self.context.tessellate(full_output.shapes);
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }

        self.renderer.update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);

        // 渲染
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("GUI Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,  // 保留场景渲染结果
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        // 清理释放的纹理
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        Ok(())
    }

    /// 获取 GUI 状态引用
    pub fn state(&self) -> &GuiState {
        &self.gui_state
    }

    /// 获取 GUI 状态可变引用
    pub fn state_mut(&mut self) -> &mut GuiState {
        &mut self.gui_state
    }
}
