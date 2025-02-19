use std::collections::{HashMap, HashSet, VecDeque};
use bevy::color::Color;
use bevy::math::{DVec3, Vec2};
use bevy::prelude::{Component, Entity, Resource, Vec3};
use bevy_reflect::Reflect;

/// Represents a single voxel with a color.
#[derive(Debug, Clone, Copy, Component, PartialEq, Default)]
pub struct Voxel {
    pub color: Color,
    pub position: Vec3,
}

/// Represents a node in the sparse voxel octree.

#[derive(Debug, Component, Clone)]
pub struct OctreeNode {
    pub children: Option<Box<[OctreeNode; 8]>>,
    pub voxel: Option<Voxel>,
    pub is_leaf: bool,
}
/// Represents the root of the sparse voxel octree.
/// Represents the root of the sparse voxel octree.
#[derive(Debug, Component, Reflect)]
#[reflect(from_reflect = false)]
pub struct SparseVoxelOctree {

    #[reflect(ignore)]
    pub root: OctreeNode,
    pub max_depth: u32,
    pub size: f32,
    pub show_wireframe: bool,
    pub show_world_grid: bool,
    pub show_chunks: bool,
    pub dirty: bool,
}

impl OctreeNode {
    /// Creates a new empty octree node.
    pub fn new() -> Self {
        Self {
            children: None,
            voxel: None,
            is_leaf: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.voxel.is_none() && self.children.is_none()
    }
}

impl Voxel {
    /// Creates a new empty octree node.
    pub fn new(color: Color) -> Self {
        Self {
            color,
            position: Vec3::ZERO
        }
    }
}


pub const NEIGHBOR_OFFSETS: [(f32, f32, f32); 6] = [
    (-1.0, 0.0, 0.0), // Left
    (1.0, 0.0, 0.0),  // Right
    (0.0, -1.0, 0.0), // Down
    (0.0, 1.0, 0.0),  // Up
    (0.0, 0.0, -1.0), // Back
    (0.0, 0.0, 1.0),  // Front
];


#[derive(Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}