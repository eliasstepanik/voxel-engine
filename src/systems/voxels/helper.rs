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


    /// Returns the size of one voxel at the given depth.
    pub fn get_spacing_at_depth(&self, depth: u32) -> f32 {
        let effective = depth.min(self.max_depth);
        self.size / (2_u32.pow(effective)) as f32
    }


    /// Center-based: [-size/2..+size/2]. Shift +half_size => [0..size], floor, shift back.
    pub fn normalize_to_voxel_at_depth(&self, position: Vec3, depth: u32) -> Vec3 {
        // Convert world coordinate to normalized [0,1] space.
        let half_size = self.size * 0.5;
        // Shift to [0, self.size]
        let shifted = (position + Vec3::splat(half_size)) / self.size;
        // Determine the number of voxels along an edge at the given depth.
        let voxel_count = 2_u32.pow(depth) as f32;
        // Get the voxel index (as a float) and then compute the center in normalized space.
        let voxel_index = (shifted * voxel_count).floor();
        let voxel_center = (voxel_index + Vec3::splat(0.5)) / voxel_count;
        voxel_center
    }
    pub fn denormalize_voxel_center(&self, voxel_center: Vec3) -> Vec3 {
        let half_size = self.size * 0.5;
        // Convert the normalized voxel center back to world space.
        voxel_center * self.size - Vec3::splat(half_size)
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
        // Define a safe inverse function to avoid division by zero.
        let safe_inv = |d: f32| if d.abs() < 1e-6 { 1e6 } else { 1.0 / d };
        let inv_dir = Vec3::new(
            safe_inv(ray.direction.x),
            safe_inv(ray.direction.y),
            safe_inv(ray.direction.z),
        );

        let t1 = (aabb.min - ray.origin) * inv_dir;
        let t2 = (aabb.max - ray.origin) * inv_dir;

        let tmin = t1.min(t2);
        let tmax = t1.max(t2);

        let t_enter = tmin.max_element();
        let t_exit = tmax.min_element();

        if t_enter <= t_exit && t_exit >= 0.0 {
            let epsilon = 1e-6;
            let mut normal = Vec3::ZERO;
            // Determine which face was hit by comparing t_enter to the computed values.
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

    /// Checks if (x,y,z) is within [-size/2..+size/2].
    pub fn contains(&self, x: f32, y: f32, z: f32) -> bool {
        let half_size = self.size / 2.0;
        let eps = 1e-6;
        (x >= -half_size - eps && x < half_size + eps)
            && (y >= -half_size - eps && y < half_size + eps)
            && (z >= -half_size - eps && z < half_size + eps)
    }

    /// Retrieve a voxel at world coordinates by normalizing and looking up.
    pub fn get_voxel_at_world_coords(&self, position: Vec3) -> Option<&Voxel> {
        let aligned = self.normalize_to_voxel_at_depth(position, self.max_depth);
        self.get_voxel_at(aligned.x, aligned.y, aligned.z)
    }

    pub fn local_to_world(&self, local_pos: Vec3) -> Vec3 {
        // Half the total octree size, used to shift the center to the origin.
        let half_size = self.size * 0.5;
        // Convert local coordinate to world space:
        // 1. Subtract 0.5 to center the coordinate at zero (range becomes [-0.5, 0.5])
        // 2. Multiply by the total size to scale into world units.
        // 3. Add half_size to shift from a center–based system to one starting at zero.
        (local_pos - Vec3::splat(0.5)) * self.size + Vec3::splat(half_size)
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