use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
pub struct MyVertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

vulkano::impl_vertex!(MyVertex, position, color);
