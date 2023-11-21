use std::ops::{Div, Sub};

use glam::{Vec2, Vec3, Vec4, Vec4Swizzles};
use nmsr_rendering::low_level::primitives::{part_primitive::PartPrimitive, vertex::Vertex};

use crate::{
    model::RenderEntry,
    shader::{fragment_shader, vertex_shader, ShaderState, VertexInput, VertexOutput},
};

impl RenderEntry {
    pub fn draw_primitives(&mut self, state: &ShaderState) {
        let vertices = self.primitive.get_vertices();
        let indices = self.primitive.get_indices();

        let triangles = indices.chunks_exact(3);
        for triangle in triangles {
            self.draw_triangle(triangle, &vertices, state);
        }
    }

    pub fn draw_triangle(&mut self, indices: &[u16], vertices: &[Vertex], state: &ShaderState) {
        
        if indices.len() != 3 {
            return;
        }

        // Our triangles are defined by three indices (clockwise)
        let va = apply_vertex_shader(vertices[indices[2] as usize], state);
        let vb = apply_vertex_shader(vertices[indices[1] as usize], state);
        let vc = apply_vertex_shader(vertices[indices[0] as usize], state);
        
        println!("Drawing triangle with vertices {va:#?}, {vb:#?}, {vc:#?}");
        
        // Next, we need to rasterize the triangle
        // We'll do this by finding the bounding box of the triangle
        // and then iterating over all pixels in that box

        // Find the bounding box (in screen space)
        let min_x = va.position.x.min(vb.position.x).min(vc.position.x).floor();
        let max_x = va.position.x.max(vb.position.x).max(vc.position.x).ceil();
        let min_y = va.position.y.min(vb.position.y).min(vc.position.y).floor();
        let max_y = va.position.y.max(vb.position.y).max(vc.position.y).ceil();

        // Convert the bounding box to actual screen coordinates
        let min_screen_x: u32 = map_float_u32(min_x, 0u32, self.size.width);
        let max_screen_x: u32 = map_float_u32(max_x, 0u32, self.size.width);
        let min_screen_y: u32 = map_float_u32(min_y, 0u32, self.size.height);
        let max_screen_y: u32 = map_float_u32(max_y, 0u32, self.size.height);

        println!(
            "min_x: {}, max_x: {}, min_y: {}, max_y: {}",
            min_screen_x, max_screen_x, min_screen_y, max_screen_y
        );

        // Iterate over all pixels in the bounding box
        for screen_y in min_screen_y..max_screen_y {
            for screen_x in min_screen_x..max_screen_x {
                // Convert the pixel coordinates to screen space
                let barycentric_coordinates = |x: f32, y: f32, a: Vec3, b: Vec3, c: Vec3| {
                    let v0 = b - a;
                    let v1 = c - a;
                    let v2 = Vec3::new(x, y, 0.0) - a;

                    let d00 = v0.dot(v0);
                    let d01 = v0.dot(v1);
                    let d11 = v1.dot(v1);
                    let d20 = v2.dot(v0);
                    let d21 = v2.dot(v1);
                    let denom = d00 * d11 - d01 * d01;

                    let v = (d11 * d20 - d01 * d21) / denom;
                    let w = (d00 * d21 - d01 * d20) / denom;
                    let u = 1.0 - v - w;                    
                    
                    Vec3::new(u, v, w)
                };

                let x = map_u32_float(screen_x, 0, self.size.width);
                let y = map_u32_float(screen_y, 0, self.size.height);
                
                
                // Compute the barycentric coordinates of the pixel
                let barycentric = barycentric_coordinates(
                    x,
                    y,
                    /* dbg! */(va.position.xyz()),
                    /* dbg! */(vb.position.xyz()),
                    /* dbg! */(vc.position.xyz()),
                );

                // If the pixel is outside the triangle, skip it
                if barycentric.x < 0.0 || barycentric.y < 0.0 || barycentric.z < 0.0 {
                    /* println! */("Skipping pixel at ({x}, {y}) because it's outside the triangle (barycentric coordinates: {barycentric:?})");
                    continue;
                }

                // Compute the depth of the pixel
                let depth = 1.0;

                // If the depth is outside the depth buffer, skip it
                if depth < 0.0 || depth > 1.0 {
                    /* println! */("Skipping pixel at ({x}, {y}) because it's outside the depth buffer");
                    continue;
                }

                // Compute the interpolated vertex attributes
                let position = barycentric.x * va.position
                    + barycentric.y * vb.position
                    + barycentric.z * vc.position;
                let tex_coord = barycentric.x * va.tex_coord
                    + barycentric.y * vb.tex_coord
                    + barycentric.z * vc.tex_coord;
                let normal = barycentric.x * va.normal
                    + barycentric.y * vb.normal
                    + barycentric.z * vc.normal;

                // Compute the color of the pixel
                let color = fragment_shader(
                    VertexOutput {
                        position,
                        tex_coord,
                        normal,
                    },
                    state,
                );

                // If the pixel is behind the depth buffer, skip it
                if depth < self.textures.depth_buffer.get_pixel(screen_x, screen_y)[0] {
                    println!("Skipping pixel at ({x}, {y}) because it's behind the depth buffer");
                    continue;
                }

                //println!("Writing pixel at ({screen_x}, {screen_y}) with color {color:?} and depth {depth}");

                // Write the pixel to the output buffer
                self.textures.output.put_pixel(
                    screen_x,
                    screen_y,
                    image::Rgba(convert_f32_slice_to_u8_slice(color)),
                );

                // Write the depth to the depth buffer
                self.textures
                    .depth_buffer
                    .put_pixel(screen_x, screen_y, image::Luma([depth]));
            }
        }
    }
}

fn map_float_u32(value: f32, min: u32, max: u32) -> u32 {
    let value = value.max(0.0).min(1.0);

    (value * (max - min) as f32 + min as f32) as u32
}

fn map_u32_float(value: u32, min: u32, max: u32) -> f32 {
    let value = value.max(min).min(max);

    (value - min) as f32 / (max - min) as f32
}

fn apply_vertex_shader(vertex: Vertex, state: &ShaderState) -> VertexOutput {
    let vertex = vertex;

    vertex_shader(
        VertexInput {
            position: vertex.position.extend(1.0),
            normal: vertex.normal,
            tex_coord: vertex.uv,
        },
        state,
    )
}

fn convert_f32_slice_to_u8_slice(slice: Vec4) -> [u8; 4] {
    let result = slice * 255.0;

    [
        result.x as u8,
        result.y as u8,
        result.z as u8,
        result.w as u8,
    ]
}
