use bevy::color::Color;
use bevy::math::DVec3;
use bevy::prelude::Vec3;
use bevy_egui::egui::Key::D;
use crate::systems::voxels::structure::{OctreeNode, Ray, SparseVoxelOctree, Voxel, AABB};


impl SparseVoxelOctree {
    pub fn ray_intersects_aabb(&self,ray: &Ray, aabb: &AABB) -> bool {
        let inv_dir = 1.0 / ray.direction;
        let t1 = (aabb.min - ray.origin) * inv_dir;
        let t2 = (aabb.max - ray.origin) * inv_dir;

        let t_min = t1.min(t2);
        let t_max = t1.max(t2);

        let t_enter = t_min.max_element();
        let t_exit = t_max.min_element();

        t_enter <= t_exit && t_exit >= 0.0
    }


    pub fn get_spacing_at_depth(&self, depth: u32) -> f32 {
        // Ensure the depth does not exceed the maximum depth
        let effective_depth = depth.min(self.max_depth);

        // Calculate the voxel size at the specified depth
        self.size / (2_u32.pow(effective_depth)) as f32
    }

    /// Normalize the world position to the nearest voxel grid position at the specified depth.
    pub fn normalize_to_voxel_at_depth(
        &self,
        world_x: f32,
        world_y: f32,
        world_z: f32,
        depth: u32,
    ) -> (f32, f32, f32) {
        // Calculate the voxel size at the specified depth
        let voxel_size = self.get_spacing_at_depth(depth) as f32;

        // Align the world position to the center of the voxel
        let aligned_x = (world_x / voxel_size).floor() * voxel_size + voxel_size / 2.0;
        let aligned_y = (world_y / voxel_size).floor() * voxel_size + voxel_size / 2.0;
        let aligned_z = (world_z / voxel_size).floor() * voxel_size + voxel_size / 2.0;

        (aligned_x, aligned_y, aligned_z)
    }

    pub fn compute_child_bounds(&self, bounds: &AABB, index: usize) -> AABB {
        let min = bounds.min;
        let max = bounds.max;
        let center = (min + max) / 2.0;

        let x_min = if (index & 1) == 0 { min.x } else { center.x };
        let x_max = if (index & 1) == 0 { center.x } else { max.x };

        let y_min = if (index & 2) == 0 { min.y } else { center.y };
        let y_max = if (index & 2) == 0 { center.y } else { max.y };

        let z_min = if (index & 4) == 0 { min.z } else { center.z };
        let z_max = if (index & 4) == 0 { center.z } else { max.z };

        let child_bounds = AABB {
            min: Vec3::new(x_min, y_min, z_min),
            max: Vec3::new(x_max, y_max, z_max),
        };

        child_bounds
    }

    pub fn ray_intersects_aabb_with_normal(
        &self,
        ray: &Ray,
        aabb: &AABB,
    ) -> Option<(f32, f32, Vec3)> {
        let inv_dir = 1.0 / ray.direction;

        let t1 = (aabb.min - ray.origin) * inv_dir;
        let t2 = (aabb.max - ray.origin) * inv_dir;

        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let t_enter = tmin.max_element();
        let t_exit = tmax.min_element();

        if t_enter <= t_exit && t_exit >= 0.0 {
            // Calculate normal based on which component contributed to t_enter
            let epsilon = 1e-6;
            let mut normal = Vec3::ZERO;

            if (t_enter - t1.x).abs() < epsilon || (t_enter - t2.x).abs() < epsilon {
                normal = Vec3::new(if ray.direction.x < 0.0 { 1.0 } else { -1.0 }, 0.0, 0.0);
            } else if (t_enter - t1.y).abs() < epsilon || (t_enter - t2.y).abs() < epsilon {
                normal = Vec3::new(0.0, if ray.direction.y < 0.0 { 1.0 } else { -1.0 }, 0.0);
            } else if (t_enter - t1.z).abs() < epsilon || (t_enter - t2.z).abs() < epsilon {
                normal = Vec3::new(0.0, 0.0, if ray.direction.z < 0.0 { 1.0 } else { -1.0 });
            }

            Some((t_enter, t_exit, normal))
        } else {
            None
        }
    }

    /// Checks if a position is within the current octree bounds.
    pub fn contains(&self, x: f32, y: f32, z: f32) -> bool {
        let half_size = self.size / 2.0;
        let epsilon = 1e-6; // Epsilon for floating-point precision

        (x >= -half_size - epsilon && x < half_size + epsilon) &&
            (y >= -half_size - epsilon && y < half_size + epsilon) &&
            (z >= -half_size - epsilon && z < half_size + epsilon)
    }

    pub fn get_voxel_at_world_coords(&self, world_x: f32, world_y: f32, world_z: f32) -> Option<&Voxel> {
        // Correct normalization: calculate the position relative to the octree's center
        let normalized_x = (world_x + (self.size / 2.0)) / self.size;
        let normalized_y = (world_y + (self.size / 2.0)) / self.size;
        let normalized_z = (world_z + (self.size / 2.0)) / self.size;

        self.get_voxel_at(normalized_x, normalized_y, normalized_z)
    }
    

    pub fn has_volume(&self, node: &OctreeNode) -> bool {
        // Check if this node is a leaf with a voxel
        if node.is_leaf && node.voxel.is_some() {
            return true;
        }

        // If the node has children, recursively check them
        if let Some(children) = &node.children {
            for child in children.iter() {
                if self.has_volume(child) {
                    return true; // If any child has a voxel, the chunk has volume
                }
            }
        }

        // If no voxel found in this node or its children
        false
    }
    

    /// Helper function to recursively traverse the octree to a specific depth.
    fn get_node_at_depth(
        node: &OctreeNode,
        x: f32,
        y: f32,
        z: f32,
        depth: u32,
    ) -> Option<&OctreeNode> {
        if depth == 0 {
            return Some(node); // We've reached the desired depth
        }

        if let Some(ref children) = node.children {
            // Determine which child to traverse into
            let epsilon = 1e-6;
            let index = ((x >= 0.5 - epsilon) as usize)
                + ((y >= 0.5 - epsilon) as usize * 2)
                + ((z >= 0.5 - epsilon) as usize * 4);

            let adjust_coord = |coord: f32| {
                if coord >= 0.5 - epsilon {
                    (coord - 0.5) * 2.0
                } else {
                    coord * 2.0
                }
            };

            // Recurse into the correct child
            Self::get_node_at_depth(
                &children[index],
                adjust_coord(x),
                adjust_coord(y),
                adjust_coord(z),
                depth - 1,
            )
        } else {
            None // Node has no children at this depth
        }
    }


}

/// Returns the (face_normal, local_offset) for the given neighbor direction.
/// - `dx, dy, dz`: The integer direction of the face (-1,0,0 / 1,0,0 / etc.)
/// - `voxel_size_f`: The world size of a single voxel (e.g. step as f32).
pub fn face_orientation(dx: f32, dy: f32, dz: f32, voxel_size_f: f32) -> (Vec3, Vec3) {
    // We'll do a match on the direction
    match (dx, dy, dz) {
        // Negative X => face normal is (-1, 0, 0), local offset is -voxel_size/2 in X
        (-1.0, 0.0, 0.0) => {
            let normal = Vec3::new(-1.0, 0.0, 0.0);
            let offset = Vec3::new(-voxel_size_f * 0.5, 0.0, 0.0);
            (normal, offset)
        }
        // Positive X
        (1.0, 0.0, 0.0) => {
            let normal = Vec3::new(1.0, 0.0, 0.0);
            let offset = Vec3::new(voxel_size_f * 0.5, 0.0, 0.0);
            (normal, offset)
        }
        // Negative Y
        (0.0, -1.0, 0.0) => {
            let normal = Vec3::new(0.0, -1.0, 0.0);
            let offset = Vec3::new(0.0, -voxel_size_f * 0.5, 0.0);
            (normal, offset)
        }
        // Positive Y
        (0.0, 1.0, 0.0) => {
            let normal = Vec3::new(0.0, 1.0, 0.0);
            let offset = Vec3::new(0.0, voxel_size_f * 0.5, 0.0);
            (normal, offset)
        }
        // Negative Z
        (0.0, 0.0, -1.0) => {
            let normal = Vec3::new(0.0, 0.0, -1.0);
            let offset = Vec3::new(0.0, 0.0, -voxel_size_f * 0.5);
            (normal, offset)
        }
        // Positive Z
        (0.0, 0.0, 1.0) => {
            let normal = Vec3::new(0.0, 0.0, 1.0);
            let offset = Vec3::new(0.0, 0.0, voxel_size_f * 0.5);
            (normal, offset)
        }
        // If the direction is not one of the 6 axis directions, you might skip or handle differently
        _ => {
            // For safety, we can panic or return a default. 
            // But typically you won't call face_orientation with an invalid direction
            panic!("Invalid face direction: ({}, {}, {})", dx, dy, dz);
        }
    }
}