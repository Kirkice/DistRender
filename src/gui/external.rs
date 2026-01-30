use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use shared_memory::{Shmem, ShmemConf};

use crate::core::{Config, SceneConfig};
use crate::gui::ipc::{GuiStatePacket, SharedGuiState, DEFAULT_SHM_NAME};

pub struct ExternalGui {
    pub shmem: Shmem,
    pub child: Option<Child>,
}

impl ExternalGui {
    pub fn try_start(config: &Config, scene: &SceneConfig) -> Option<Self> {
        // 先确保旧的 GUI 进程被终止
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(&["/F", "/IM", "dist_render_gui.exe"])
                .output();
            // 等待进程完全终止和资源释放
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        let packet0 = GuiStatePacket {
            clear_color: scene.clear_color,
            light_intensity: scene.light.intensity,
            light_direction: scene.light.transform.rotation,
            model_position: scene.model.transform.position,
            model_rotation: scene.model.transform.rotation,
            model_scale: scene.model.transform.scale,
            camera_fov: scene.camera.fov,
            camera_near: scene.camera.near_clip,
            camera_far: scene.camera.far_clip,
        };

        let size = SharedGuiState::MAGIC_SIZE;
        
        // 在 Windows 上，命名共享内存在所有进程关闭后仍然存在
        // 策略：先尝试打开已存在的，如果不存在则创建新的
        // 无论哪种情况都重新初始化数据
        let shmem = match ShmemConf::new().os_id(DEFAULT_SHM_NAME).open() {
            Ok(shmem) => {
                tracing::debug!("Opened existing shared memory, reinitializing...");
                unsafe {
                    let ptr = shmem.as_ptr() as *mut SharedGuiState;
                    ptr.write(SharedGuiState::new_init(packet0));
                }
                tracing::debug!("Reinitialized existing shared memory");
                shmem
            }
            Err(_) => {
                // 不存在，创建新的
                tracing::debug!("Creating new shared memory...");
                match ShmemConf::new().os_id(DEFAULT_SHM_NAME).size(size).create() {
                    Ok(shmem) => {
                        unsafe {
                            let ptr = shmem.as_ptr() as *mut SharedGuiState;
                            ptr.write(SharedGuiState::new_init(packet0));
                        }
                        tracing::debug!("Created new shared memory");
                        shmem
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create shared memory: {}", e);
                        return None;
                    }
                }
            }
        };

        let gui_exe = find_gui_exe();
        let child = match gui_exe {
            Some(path) => {
                let mut cmd = Command::new(path);
                cmd.arg("--wgpu");
                cmd.stdin(Stdio::null());
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::null());

                match cmd.spawn() {
                    Ok(child) => {
                        tracing::info!(backend = %config.graphics.backend.name(), "External GUI process started");
                        Some(child)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to spawn external GUI process: {}", e);
                        None
                    }
                }
            }
            None => {
                tracing::warn!("dist_render_gui executable not found");
                None
            }
        };

        Some(Self { shmem, child })
    }

    pub fn read_packet(&self) -> GuiStatePacket {
        let shared = unsafe { &*(self.shmem.as_ptr() as *const SharedGuiState) };
        shared.read_latest()
    }
}

impl Drop for ExternalGui {
    fn drop(&mut self) {
        tracing::debug!("Dropping ExternalGui, cleaning up resources...");
        
        // 终止外部 GUI 进程
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
            tracing::debug!("External GUI process terminated");
        }
        
        // 共享内存会在 Shmem drop 时自动清理
        tracing::debug!("ExternalGui dropped successfully");
    }
}

fn find_gui_exe() -> Option<PathBuf> {
    // Prefer "B": same directory as current executable.
    if let Ok(this_exe) = std::env::current_exe() {
        if let Some(dir) = this_exe.parent() {
            let p = dir.join(gui_exe_name());
            if p.is_file() {
                return Some(p);
            }
        }
    }

    // Fallback "A": target/debug/dist_render_gui(.exe) relative to current working directory.
    let p = Path::new("target").join("debug").join(gui_exe_name());
    if p.is_file() {
        return Some(p);
    }

    None
}

fn gui_exe_name() -> &'static str {
    if cfg!(windows) { "dist_render_gui.exe" } else { "dist_render_gui" }
}
