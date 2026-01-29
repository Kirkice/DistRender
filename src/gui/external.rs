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
        let shmem = match ShmemConf::new().os_id(DEFAULT_SHM_NAME).size(size).create() {
            Ok(shmem) => {
                unsafe {
                    let ptr = shmem.as_ptr() as *mut SharedGuiState;
                    ptr.write(SharedGuiState::new_init(packet0));
                }
                shmem
            }
            Err(_) => match ShmemConf::new().os_id(DEFAULT_SHM_NAME).open() {
                Ok(shmem) => shmem,
                Err(e) => {
                    tracing::warn!("Failed to create/open shared memory for external GUI: {}", e);
                    return None;
                }
            },
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
