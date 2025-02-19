use std::collections::{HashMap, HashSet};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::math::{DQuat, DVec3};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use crate::systems::voxels::structure::{OctreeNode, Ray, SparseVoxelOctree, Voxel, AABB, NEIGHBOR_OFFSETS};

impl SparseVoxelOctree {
    /// Creates a new octree with the specified max depth, size, and wireframe visibility.
    pub fn new(max_depth: u32, size: f32, show_wireframe: bool, show_world_grid: bool, show_chunks: bool) -> Self {
        Self {
            root: OctreeNode::new(),
            max_depth,
            size,
            show_wireframe,
            show_world_grid,
            show_chunks,
            dirty: true,
        }
    }

    pub fn insert(&mut self, world_x: f32, world_y: f32, world_z: f32, voxel: Voxel) {
        // Normalize the world coordinates to the nearest voxel grid position
        let (aligned_x, aligned_y, aligned_z) = self.normalize_to_voxel_at_depth(world_x, world_y, world_z, self.max_depth);

        // Iteratively expand the root to include the voxel position
        while !self.contains(aligned_x, aligned_y, aligned_z) {
            self.expand_root(aligned_x, aligned_y, aligned_z);
        }

        // Correct normalization: calculate the position relative to the octree's center
        let normalized_x = (aligned_x + (self.size / 2.0)) / self.size;
        let normalized_y = (aligned_y + (self.size / 2.0)) / self.size;
        let normalized_z = (aligned_z + (self.size / 2.0)) / self.size;

        // Insert the voxel with its world position
        let mut voxel_with_position = voxel;
        voxel_with_position.position = Vec3::new(world_x as f32, world_y as f32, world_z as f32);
        

        self.dirty = true;


        SparseVoxelOctree::insert_recursive(&mut self.root, normalized_x, normalized_y, normalized_z, voxel_with_position, self.max_depth);
    }

    fn insert_recursive(node: &mut OctreeNode, x: f32, y: f32, z: f32, voxel: Voxel, depth: u32) {
        if depth == 0 {
            node.voxel = Some(voxel);
            node.is_leaf = true;
            return;
        }

        let epsilon = 1e-6; // Epsilon for floating-point precision

        let index = ((x >= 0.5 - epsilon) as usize) + ((y >= 0.5 - epsilon) as usize * 2) + ((z >= 0.5 - epsilon) as usize * 4);

        if node.children.is_none() {
            node.children = Some(Box::new(core::array::from_fn(|_| OctreeNode::new())));
            node.is_leaf = false;
        }

        if let Some(ref mut children) = node.children {
            let adjust_coord = |coord: f32| {
                if coord >= 0.5 - epsilon {
                    (coord - 0.5) * 2.0
                } else {
                    coord * 2.0
                }
            };
            SparseVoxelOctree::insert_recursive(&mut children[index], adjust_coord(x), adjust_coord(y), adjust_coord(z), voxel, depth - 1);
        }
    }

    pub fn remove(&mut self, world_x: f32, world_y: f32, world_z: f32) {
        // Normalize the world coordinates to the nearest voxel grid position
        let (aligned_x, aligned_y, aligned_z) =
            self.normalize_to_voxel_at_depth(world_x, world_y, world_z, self.max_depth);

        // Correct normalization: calculate the position relative to the octree's center
        let normalized_x = (aligned_x + (self.size / 2.0)) / self.size;
        let normalized_y = (aligned_y + (self.size / 2.0)) / self.size;
        let normalized_z = (aligned_z + (self.size / 2.0)) / self.size;

        self.dirty = true;


        // Call the recursive remove function
        Self::remove_recursive(&mut self.root, normalized_x, normalized_y, normalized_z, self.max_depth);
    }

    fn remove_recursive(node: &mut OctreeNode, x: f32, y: f32, z: f32, depth: u32) -> bool {
        if depth == 0 {
            // This is the leaf node where the voxel should be
            if node.voxel.is_some() {
                node.voxel = None;
                node.is_leaf = false;
                // Since we've removed the voxel and there are no children, this node can be pruned
                return true;
            } else {
                // There was no voxel here
                return false;
            }
        }

        if node.children.is_none() {
            // No children to traverse, voxel not found
            return false;
        }

        let epsilon = 1e-6; // Epsilon for floating-point precision
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

        let child = &mut node.children.as_mut().unwrap()[index];

        let should_prune_child = Self::remove_recursive(
            child,
            adjust_coord(x),
            adjust_coord(y),
            adjust_coord(z),
            depth - 1,
        );

        if should_prune_child {
            // Remove the child node
            node.children.as_mut().unwrap()[index] = OctreeNode::new();
        }

        // After removing the child, check if all children are empty
        let all_children_empty = node.children.as_ref().unwrap().iter().all(|child| child.is_empty());

        if all_children_empty {
            // Remove the children array
            node.children = None;
            node.is_leaf = true; // Now this node becomes a leaf
            // If this node has no voxel and no children, it can be pruned
            return node.voxel.is_none();
        } else {
            return false;
        }
    }


    fn expand_root(&mut self, x: f32, y: f32, z: f32) {
        let new_size = self.size * 2.0;
        let new_depth = self.max_depth + 1;

        // Create a new root node with 8 children
        let mut new_root = OctreeNode::new();
        new_root.children = Some(Box::new(core::array::from_fn(|_| OctreeNode::new())));

        // The old root had 8 children; move each child to the correct new position
        if let Some(old_children) = self.root.children.take() {
            for (i, old_child) in old_children.iter().enumerate() {
                // Determine which child of the new root the old child belongs in
                let offset_x = if (i & 1) == 1 { 1 } else { 0 };
                let offset_y = if (i & 2) == 2 { 1 } else { 0 };
                let offset_z = if (i & 4) == 4 { 1 } else { 0 };

                let new_index = offset_x + (offset_y * 2) + (offset_z * 4);

                // Now, move the old child into the correct new child of the new root
                let new_child = &mut new_root.children.as_mut().unwrap()[new_index];

                // Create new children for the new child if necessary
                if new_child.children.is_none() {
                    new_child.children = Some(Box::new(core::array::from_fn(|_| OctreeNode::new())));
                }

                // Place the old child in the correct "facing" position in the new child
                let facing_x = if offset_x == 1 { 0 } else { 1 };
                let facing_y = if offset_y == 1 { 0 } else { 1 };
                let facing_z = if offset_z == 1 { 0 } else { 1 };

                let facing_index = facing_x + (facing_y * 2) + (facing_z * 4);
                new_child.children.as_mut().unwrap()[facing_index] = old_child.clone();
            }
        }

        self.root = new_root;
        self.size = new_size;
        self.max_depth = new_depth;
    }


    /// Traverse the octree and collect voxel data.
    pub fn traverse(&self) -> Vec<(f32, f32, f32, Color, u32)> {
        let mut voxels = Vec::new();
        Self::traverse_recursive(&self.root, 0.0, 0.0, 0.0, 1.0, 0, &mut voxels);
        voxels
    }

    fn traverse_recursive(
        node: &OctreeNode,
        x: f32,
        y: f32,
        z: f32,
        size: f32,
        depth: u32,
        voxels: &mut Vec<(f32, f32, f32, Color, u32)>,
    ) {
        if node.is_leaf/* && !node.is_constant*/ {
            if let Some(voxel) = node.voxel {
                voxels.push((x, y, z, voxel.color, depth));
            }
        }

        if let Some(ref children) = node.children {
            let half_size = size / 2.0;
            for (i, child) in children.iter().enumerate() {
                let offset = |bit: usize| if (i & bit) == bit { half_size } else { 0.0 };
                Self::traverse_recursive(
                    child,
                    x + offset(1),
                    y + offset(2),
                    z + offset(4),
                    half_size,
                    depth + 1,
                    voxels,
                );
            }
        }
    }


    /// Retrieves a reference to the voxel at the given normalized coordinates and depth, if it exists.
    pub fn get_voxel_at(&self, x: f32, y: f32, z: f32) -> Option<&Voxel> {
        Self::get_voxel_recursive(&self.root, x, y, z)
    }

    fn get_voxel_recursive(
        node: &OctreeNode,
        x: f32,
        y: f32,
        z: f32,
    ) -> Option<&Voxel> {
        if node.is_leaf {
            return node.voxel.as_ref();
        }

        if let Some(ref children) = node.children {
            let epsilon = 1e-6; // Epsilon for floating-point precision
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

            Self::get_voxel_recursive(
                &children[index],
                adjust_coord(x),
                adjust_coord(y),
                adjust_coord(z),
            )
        } else {
            None
        }
    }

    /// Checks if there is a neighbor voxel at the specified direction from the given world coordinates at the specified depth.
    /// The offsets are directions (-1, 0, 1) for x, y, z.
    pub fn has_neighbor(
        &self,
        world_x: f32,
        world_y: f32,
        world_z: f32,
        offset_x: i32,
        offset_y: i32,
        offset_z: i32,
        depth: u32,
    ) -> bool {
        // Normalize the world coordinates to the nearest voxel grid position at the specified depth
        let (aligned_x, aligned_y, aligned_z) =
            self.normalize_to_voxel_at_depth(world_x, world_y, world_z, depth);

        // Calculate the voxel size at the specified depth
        let voxel_size = self.get_spacing_at_depth(depth);

        // Calculate the neighbor's world position
        let neighbor_x = aligned_x + (offset_x as f32) * voxel_size;
        let neighbor_y = aligned_y + (offset_y as f32) * voxel_size;
        let neighbor_z = aligned_z + (offset_z as f32) * voxel_size;

        // Check if the neighbor position is within bounds
        if !self.contains(neighbor_x, neighbor_y, neighbor_z) {
            return false;
        }

        // Get the voxel in the neighboring position
        self.get_voxel_at_world_coords(neighbor_x, neighbor_y, neighbor_z)
            .is_some()
    }


    /// Performs a raycast against the octree and returns the first intersected voxel.
    pub fn raycast(&self, ray: &Ray) -> Option<(f32, f32, f32, u32, Vec3)> {
        // Start from the root node
        let half_size = self.size / 2.0;
        let root_bounds = AABB {
            min: Vec3::new(-half_size as f32, -half_size as f32, -half_size as f32),
            max: Vec3::new(half_size as f32, half_size as f32, half_size as f32),
        };
        self.raycast_recursive(
            &self.root,
            ray,
            &root_bounds,
            0,
        )
    }

    fn raycast_recursive(
        &self,
        node: &OctreeNode,
        ray: &Ray,
        bounds: &AABB,
        depth: u32,
    ) -> Option<(f32, f32, f32, u32, Vec3)> {
        // Check if the ray intersects this node's bounding box
        if let Some((t_enter, _, normal)) = self.ray_intersects_aabb_with_normal(ray, bounds) {
            // If this is a leaf node and contains a voxel, return it
            if node.is_leaf && node.voxel.is_some() {
                // Compute the exact hit position
                let hit_position = ray.origin + ray.direction * t_enter;

                // Return the hit position along with depth and normal
                return Some((
                    hit_position.x as f32,
                    hit_position.y as f32,
                    hit_position.z as f32,
                    depth,
                    normal,
                ));
            }

            // If the node has children, traverse them
            if let Some(ref children) = node.children {
                // For each child, compute its bounding box and recurse
                let mut hits = Vec::new();
                for (i, child) in children.iter().enumerate() {
                    let child_bounds = self.compute_child_bounds(bounds, i);
                    if let Some(hit) = self.raycast_recursive(child, ray, &child_bounds, depth + 1) {
                        hits.push(hit);
                    }
                }
                // Return the closest hit, if any
                if !hits.is_empty() {
                    hits.sort_by(|a, b| {
                        let dist_a = ((a.0 as f32 - ray.origin.x).powi(2)
                            + (a.1 as f32 - ray.origin.y).powi(2)
                            + (a.2 as f32 - ray.origin.z).powi(2))
                            .sqrt();
                        let dist_b = ((b.0 as f32 - ray.origin.x).powi(2)
                            + (b.1 as f32 - ray.origin.y).powi(2)
                            + (b.2 as f32 - ray.origin.z).powi(2))
                            .sqrt();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });
                    return Some(hits[0]);
                }
            }
        }

        None
    }
    
}

