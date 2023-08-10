use glam::Vec3;

use crate::low_level::primitives::part_primitive::PartPrimitive;
use crate::low_level::primitives::vertex::Vertex;

use super::vertex::VertexUvCoordinates;



pub struct Quad {
    top_left: Vertex,
    top_right: Vertex,
    bottom_left: Vertex,
    bottom_right: Vertex,
}

impl Quad {
    /// Create a new quad with the given vertices
    pub fn new_from_vec(
        top_left: Vertex,
        top_right: Vertex,
        bottom_left: Vertex,
        bottom_right: Vertex,
    ) -> Self {
        Quad {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    /// Create a new quad with the given vertex positions and uv coordinates
    pub fn new(
        top_left: Vec3,
        top_right: Vec3,
        bottom_left: Vec3,
        bottom_right: Vec3,
        top_left_uv: VertexUvCoordinates,
        bottom_right_uv: VertexUvCoordinates,
    ) -> Self {
        let normal = (top_right - top_left).cross(bottom_left - top_left).normalize();
        
        Self::new_with_normal(top_left, top_right, bottom_left, bottom_right, top_left_uv, bottom_right_uv, normal)
    }
    
    pub fn new_with_normal(
        top_left: Vec3,
        top_right: Vec3,
        bottom_left: Vec3,
        bottom_right: Vec3,
        top_left_uv: VertexUvCoordinates,
        bottom_right_uv: VertexUvCoordinates,
        normal: Vec3,
    ) -> Self {
        Quad {
            top_left: Vertex::new(top_left, top_left_uv, normal),
            top_right: Vertex::new(top_right, [bottom_right_uv.x, top_left_uv.y].into(), normal),
            bottom_left: Vertex::new(bottom_left, [top_left_uv.x, bottom_right_uv.y].into(), normal),
            bottom_right: Vertex::new(bottom_right, bottom_right_uv, normal),
        }
    }
}

impl PartPrimitive for Quad {
    fn get_vertices(&self) -> Vec<Vertex> {
        vec![
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
        ]
    }

    fn get_indices(&self) -> Vec<u16> {
        // We're going in clockwise order
        vec![
            // First triangle (bottom left, top left, bottom right)
            2, 0, 3, // Second triangle (top left, top right, bottom right)
            0, 1, 3,
        ]
    }
}
