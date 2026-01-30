#![allow(dead_code)]
use bytemuck::{Pod, Zeroable};
use crate::math::Vector3;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, Pod, Zeroable)]
pub struct MyVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

impl MyVertex {
    pub fn from_vectors(position: Vector3, normal: Vector3, color: Vector3) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            normal: [normal.x, normal.y, normal.z],
            color: [color.x, color.y, color.z],
        }
    }

    pub fn new(px: f32, py: f32, pz: f32, nx: f32, ny: f32, nz: f32, r: f32, g: f32, b: f32) -> Self {
        Self {
            position: [px, py, pz],
            normal: [nx, ny, nz],
            color: [r, g, b],
        }
    }
}

pub fn create_default_triangle() -> [MyVertex; 3] {
    [
        MyVertex::new(0.0, 0.5, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0),
        MyVertex::new(0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0),
        MyVertex::new(-0.5, -0.5, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0),
    ]
}

pub use crate::geometry::vertex::Vertex as GeometryVertex;

pub fn convert_geometry_vertex(geo_vertex: &GeometryVertex) -> MyVertex {
    MyVertex {
        position: geo_vertex.position,
        normal: geo_vertex.normal,
        color: [1.0, 1.0, 1.0],
    }
}

vulkano::impl_vertex!(MyVertex, position, normal, color);
vulkano::impl_vertex!(GeometryVertex, position, normal, texcoord, tangent);
