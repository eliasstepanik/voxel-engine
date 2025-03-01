use std::collections::{HashMap, HashSet};
use bevy::asset::Assets;
use bevy::color::Color;
use bevy::math::{DQuat, DVec3};
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::render::render_asset::RenderAssetUsages;
use crate::systems::voxels::structure::{DirtyVoxel, OctreeNode, Ray, SparseVoxelOctree, Voxel, AABB, NEIGHBOR_OFFSETS};

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
            dirty: Vec::new(),
        }
    }
    pub fn insert(&mut self, position: Vec3, voxel: Voxel) {
        // Align to the center of the voxel at max_depth
        let mut aligned = self.normalize_to_voxel_at_depth(position, self.max_depth);
        let mut world_center = self.denormalize_voxel_center(aligned);

        // Expand as needed using the denormalized position.
        while !self.contains(world_center.x, world_center.y, world_center.z) {
            self.expand_root(world_center.x, world_center.y, world_center.z);
            // Recompute aligned and world_center after expansion.
            aligned = self.normalize_to_voxel_at_depth(position, self.max_depth);
            world_center = self.denormalize_voxel_center(aligned);
        }

        let dirty_voxel = DirtyVoxel{
            position: aligned,
        };
        self.dirty.push(dirty_voxel);


        Self::insert_recursive(&mut self.root, aligned, voxel, self.max_depth);
    }

    fn insert_recursive(node: &mut OctreeNode, position: Vec3, voxel: Voxel, depth: u32) {
        if depth == 0 {
            node.voxel = Some(voxel);
            node.is_leaf = true;
            return;
        }
        let epsilon = 1e-6;
        // Determine octant index by comparing with 0.5
        let index = ((position.x >= 0.5 - epsilon) as usize)
            + ((position.y >= 0.5 - epsilon) as usize * 2)
            + ((position.z >= 0.5 - epsilon) as usize * 4);

        // If there are no children, create them.
        if node.children.is_none() {
            node.children = Some(Box::new(core::array::from_fn(|_| OctreeNode::new())));
            node.is_leaf = false;
        }
        if let Some(ref mut children) = node.children {
            // Adjust coordinate into the child’s [0, 1] range.
            let adjust_coord = |coord: f32| {
                if coord >= 0.5 - epsilon {
                    (coord - 0.5) * 2.0
                } else {
                    coord * 2.0
                }
            };
            let child_pos = Vec3::new(
                adjust_coord(position.x),
                adjust_coord(position.y),
                adjust_coord(position.z),
            );
            Self::insert_recursive(&mut children[index], child_pos, voxel, depth - 1);
        }
    }

    pub fn remove(&mut self, position: Vec3) {
        let aligned = self.normalize_to_voxel_at_depth(position, self.max_depth);

        let dirty_voxel = DirtyVoxel{
            position: aligned,
        };
        self.dirty.push(dirty_voxel);

        Self::remove_recursive(&mut self.root, aligned.x, aligned.y, aligned.z, self.max_depth);
    }

    fn remove_recursive(
        node: &mut OctreeNode,
        x: f32,
        y: f32,
        z: f32,
        depth: u32,
    ) -> bool {
        if depth == 0 {
            if node.voxel.is_some() {
                node.voxel = None;
                node.is_leaf = false;
                return true;
            } else {
                return false;
            }
        }

        if node.children.is_none() {
            return false;
        }
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

        let child = &mut node.children.as_mut().unwrap()[index];
        let should_prune_child = Self::remove_recursive(
            child,
            adjust_coord(x),
            adjust_coord(y),
            adjust_coord(z),
            depth - 1,
        );

        if should_prune_child {
            // remove the child node
            node.children.as_mut().unwrap()[index] = OctreeNode::new();
        }

        // Check if all children are empty
        let all_children_empty = node
            .children
            .as_ref()
            .unwrap()
            .iter()
            .all(|child| child.is_empty());

        if all_children_empty {
            node.children = None;
            node.is_leaf = true;
            return node.voxel.is_none();
        }
        false
    }


    fn expand_root(&mut self, _x: f32, _y: f32, _z: f32) {
        info!("Root expanding ...");
        // Save the old root and its size.
        let old_root = std::mem::replace(&mut self.root, OctreeNode::new());
        let old_size = self.size;

        // Update the octree's size and depth.
        self.size *= 2.0;
        self.max_depth += 1;

        // Reinsert each voxel from the old tree.
        let voxels = Self::collect_voxels_from_node(&old_root, old_size);
        for (world_pos, voxel, _depth) in voxels {
            self.insert(world_pos, voxel);
        }
    }

    /// Helper: Collect all voxels from a given octree node recursively.
    /// The coordinate system here assumes the node covers [–old_size/2, +old_size/2] in each axis.
    fn collect_voxels_from_node(node: &OctreeNode, old_size: f32) -> Vec<(Vec3, Voxel, u32)> {
        let mut voxels = Vec::new();
        Self::collect_voxels_recursive(node, -old_size / 2.0, -old_size / 2.0, -old_size / 2.0, old_size, 0, &mut voxels);
        voxels
    }

    fn collect_voxels_recursive(
        node: &OctreeNode,
        x: f32,
        y: f32,
        z: f32,
        size: f32,
        depth: u32,
        out: &mut Vec<(Vec3, Voxel, u32)>,
    ) {
        if node.is_leaf {
            if let Some(voxel) = node.voxel {
                // Compute the center of this node's region.
                let center = Vec3::new(x + size / 2.0, y + size / 2.0, z + size / 2.0);
                out.push((center, voxel, depth));
            }
        }
        if let Some(children) = &node.children {
            let half = size / 2.0;
            for (i, child) in children.iter().enumerate() {
                let offset_x = if (i & 1) != 0 { half } else { 0.0 };
                let offset_y = if (i & 2) != 0 { half } else { 0.0 };
                let offset_z = if (i & 4) != 0 { half } else { 0.0 };
                Self::collect_voxels_recursive(child, x + offset_x, y + offset_y, z + offset_z, half, depth + 1, out);
            }
        }
    }



    pub fn traverse(&self) -> Vec<(Vec3, Color, u32)> {
        let mut voxels = Vec::new();
        // Start at the normalized center (0.5, 0.5, 0.5) rather than (0,0,0)
        Self::traverse_recursive(
            &self.root,
            Vec3::splat(0.5), // normalized center of the root cell
            1.0,              // full normalized cell size
            0,
            &mut voxels,
            self,
        );
        voxels
    }

    fn traverse_recursive(
        node: &OctreeNode,
        local_center: Vec3,
        size: f32,
        depth: u32,
        out: &mut Vec<(Vec3, Color, u32)>,
        octree: &SparseVoxelOctree,
    ) {
        // If a leaf contains a voxel, record its world-space center
        if node.is_leaf {
            if let Some(voxel) = node.voxel {
                out.push((octree.denormalize_voxel_center(local_center), voxel.color, depth));
            }
        }

        // If the node has children, subdivide the cell into 8 subcells.
        if let Some(ref children) = node.children {
            let offset = size / 4.0;  // child center offset from parent center
            let new_size = size / 2.0;  // each child cell's size in normalized space
            for (i, child) in children.iter().enumerate() {
                // Compute each axis' offset: use +offset if the bit is set, else -offset.
                let dx = if (i & 1) != 0 { offset } else { -offset };
                let dy = if (i & 2) != 0 { offset } else { -offset };
                let dz = if (i & 4) != 0 { offset } else { -offset };
                let child_center = local_center + Vec3::new(dx, dy, dz);

                Self::traverse_recursive(child, child_center, new_size, depth + 1, out, octree);
            }
        }
    }



    /// Retrieve a voxel from the octree if it exists (x,y,z in [-0.5..+0.5] range).
    pub fn get_voxel_at(&self, x: f32, y: f32, z: f32) -> Option<&Voxel> {
        Self::get_voxel_recursive(&self.root, x, y, z)
    }

    fn get_voxel_recursive(node: &OctreeNode, x: f32, y: f32, z: f32) -> Option<&Voxel> {
        if node.is_leaf {
            return node.voxel.as_ref();
        }
        if let Some(children) = &node.children {
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
        position: Vec3,
        offset_x: i32,
        offset_y: i32,
        offset_z: i32,
        depth: u32,
    ) -> bool {
        let aligned = self.normalize_to_voxel_at_depth(position, depth);
        let voxel_count = 2_u32.pow(depth) as f32;
        // Normalized voxel size is 1/voxel_count
        let norm_voxel_size = 1.0 / voxel_count;

        let neighbor = Vec3::new(
            aligned.x + (offset_x as f32) * norm_voxel_size,
            aligned.y + (offset_y as f32) * norm_voxel_size,
            aligned.z + (offset_z as f32) * norm_voxel_size,
        );

        // Convert the normalized neighbor coordinate back to world space
        let half_size = self.size * 0.5;
        let neighbor_world = neighbor * self.size - Vec3::splat(half_size);

        if !self.contains(neighbor_world.x, neighbor_world.y, neighbor_world.z) {
            return false;
        }

        self.get_voxel_at_world_coords(neighbor_world).is_some()
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

