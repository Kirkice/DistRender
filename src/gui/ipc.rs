use std::mem;
use std::sync::atomic::{AtomicU32, Ordering};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GuiStatePacket {
    pub clear_color: [f32; 4],

    pub light_intensity: f32,
    pub light_direction: [f32; 3],

    pub model_position: [f32; 3],
    pub model_rotation: [f32; 3],
    pub model_scale: [f32; 3],

    pub camera_fov: f32,
    pub camera_near: f32,
    pub camera_far: f32,
}

#[repr(C)]
pub struct SharedGuiState {
    pub seq: AtomicU32,
    pub _padding: [u32; 3],

    pub a: GuiStatePacket,
    pub b: GuiStatePacket,
}

impl SharedGuiState {
    pub const MAGIC_SIZE: usize = mem::size_of::<SharedGuiState>();

    pub fn new_init(packet: GuiStatePacket) -> Self {
        Self {
            seq: AtomicU32::new(0),
            _padding: [0; 3],
            a: packet,
            b: packet,
        }
    }

    pub fn write_latest(&self, packet: GuiStatePacket) {
        let next = self.seq.load(Ordering::Relaxed).wrapping_add(1);
        if next & 1 == 0 {
            unsafe {
                let dst = &self.a as *const GuiStatePacket as *mut GuiStatePacket;
                dst.write(packet);
            }
        } else {
            unsafe {
                let dst = &self.b as *const GuiStatePacket as *mut GuiStatePacket;
                dst.write(packet);
            }
        }
        self.seq.store(next, Ordering::Release);
    }

    pub fn read_latest(&self) -> GuiStatePacket {
        loop {
            let s0 = self.seq.load(Ordering::Acquire);
            let packet = if s0 & 1 == 0 { self.a } else { self.b };
            let s1 = self.seq.load(Ordering::Acquire);
            if s0 == s1 {
                return packet;
            }
        }
    }
}

pub const DEFAULT_SHM_NAME: &str = "dist_render_gui_state_v1";
