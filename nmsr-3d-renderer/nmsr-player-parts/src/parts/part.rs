use crate::parts::part::Part::{Cube, Quad};
use crate::parts::uv::{CubeFaceUvs, FaceUv};
use crate::types::PlayerPartTextureType;
use glam::Vec3;

#[derive(Debug, Copy, Clone)]
pub struct PartAnchorInfo {
    pub anchor: MinecraftPosition,
}

#[derive(Debug, Copy, Clone)]
pub enum Part {
    /// Represents a cube as a part of a player model.
    Cube {
        position: MinecraftPosition,
        size: MinecraftPosition,
        rotation: MinecraftPosition,
        face_uvs: CubeFaceUvs,
        texture: PlayerPartTextureType,
        anchor: Option<PartAnchorInfo>,
    },
    /// Represents a quad as a part of a player model.
    Quad {
        position: MinecraftPosition,
        size: MinecraftPosition,
        rotation: MinecraftPosition,
        face_uv: FaceUv,
        texture: PlayerPartTextureType,
        anchor: Option<PartAnchorInfo>,
    },
}

impl Part {
    /// Creates a new cube part.
    ///
    /// # Arguments
    ///
    /// * `pos`: The position of the cube. [x, y, z]
    /// * `size`: The size of the cube. [x, y, z]
    /// * `uvs`: The UVs of the cube.
    /// UVs are in the following order: [North, South, East, West, Up, Down]
    /// Each UV is in the following order: [Top left, Bottom right]
    ///
    /// returns: [Part]
    pub fn new_cube(
        texture: PlayerPartTextureType,
        pos: [i32; 3],
        size: [u32; 3],
        uvs: [[u16; 4]; 6],
    ) -> Self {
        Cube {
            position: MinecraftPosition::new(pos[0] as f32, pos[1] as f32, pos[2] as f32),
            size: MinecraftPosition::new(size[0] as f32, size[1] as f32, size[2] as f32),
            rotation: MinecraftPosition::ZERO,
            face_uvs: uvs.into(),
            texture,
            anchor: None,
        }
    }

    pub fn expand(&self, amount: f32) -> Self {
        let mut new_part = *self;
        let amount = amount * 2.0;

        match new_part {
            Cube {
                ref mut size,
                ref mut position,
                ..
            } => {
                // Increase the size of the cube by the amount specified.
                *size += amount;

                // Fix the position of the cube so that it is still centered.
                *position -= amount / 2.0;
            }
            Quad {
                ref mut size,
                ref mut position,
                ..
            } => {
                // Increase the size of the quad by the amount specified.
                *size += amount;
            }
        }

        new_part
    }
    
    pub fn get_anchor(&self) -> Option<PartAnchorInfo> {
        match self {
            Cube { anchor, .. } => *anchor,
            Quad { anchor, .. } => *anchor,
        }
    }
    
    pub fn set_anchor(&mut self, anchor: Option<PartAnchorInfo>) {
        match self {
            Cube { anchor: ref mut a, .. } => *a = anchor,
            Quad { anchor: ref mut a, .. } => *a = anchor,
        }
    }
    
    pub fn get_rotation(&self) -> MinecraftPosition {
        match self {
            Cube { rotation, .. } => *rotation,
            Quad { rotation, .. } => *rotation,
        }
    }
    
    pub fn rotation_mut(&mut self) -> &mut MinecraftPosition {
        match self {
            Cube { rotation, .. } => rotation,
            Quad { rotation, .. } => rotation,
        }
    }
    
    pub fn set_rotation(&mut self, rotation: MinecraftPosition) {
        match self {
            Cube { rotation: ref mut r, .. } => *r = rotation,
            Quad { rotation: ref mut r, .. } => *r = rotation,
        }
    }

    pub fn get_size(&self) -> MinecraftPosition {
        match self {
            Cube { size, .. } => *size,
            Quad { size, .. } => *size,
        }
    }

    pub fn get_position(&self) -> MinecraftPosition {
        match self {
            Cube { position, .. } => *position,
            Quad { position, .. } => *position,
        }
    }

    pub fn get_texture(&self) -> PlayerPartTextureType {
        match self {
            Cube { texture, .. } => *texture,
            Quad { texture, .. } => *texture,
        }
    }
}

/// A position in 3D space.
///
/// Minecraft coordinates are structured as follows:
/// - +X is east / -X is west
/// - +Y is up / -Y is down
/// - +Z is south / -Z is north
pub(crate) type MinecraftPosition = Vec3;