use bevy::color::palettes::basic::{BLACK, RED, YELLOW};
use bevy::color::palettes::css::GREEN;
use bevy::math::{DQuat, Vec3};
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy_egui::egui::emath::Numeric;
use crate::systems::camera_system::Selector;
use crate::systems::voxels::structure::{OctreeNode, SparseVoxelOctree};

/// Visualize each node of the octree as a scaled cuboid, **center-based**.
/// `octree_tf.translation` is the world-space center of the root bounding box.
pub fn visualize_octree_system(
    mut gizmos: Gizmos,
    octree_query: Query<(&SparseVoxelOctree, &Transform)>,
) {
    for (octree, octree_tf) in octree_query.iter() {
        // The root node covers [-size/2..+size/2], so half_size is:
        let half_size = octree.size * 0.5;

        // Draw a translucent cuboid for the root
        gizmos.cuboid(
            Transform::from_translation(octree_tf.translation)
                .with_scale(Vec3::splat(octree.size)),
            Color::rgba(1.0, 1.0, 0.0, 0.15),
        );

        // Recursively draw children:
        // Start from depth=0. The node at depth=0 has bounding side = octree.size.
        visualize_recursive_center(
            &mut gizmos,
            &octree.root,
            octree_tf.translation, // center of root in world
            octree.size,
            0,
            octree.max_depth,
        );
    }
}

/// Recursively draws cuboids for each node. 
/// We follow the same indexing as insert_recursive, i.e. bit patterns:
/// i=0 => child in (-x,-y,-z) quadrant,
/// i=1 => (+x,-y,-z), i=2 => (-x,+y,-z), etc.
fn visualize_recursive_center(
    gizmos: &mut Gizmos,
    node: &OctreeNode,
    parent_center: Vec3,
    parent_size: f32,
    depth: u32,
    max_depth: u32,
) {
    if depth >= max_depth {
        return;
    }
    if let Some(children) = &node.children {
        // Each child is half the parent’s size
        let child_size = parent_size * 0.5;
        let half = child_size * 0.5;

        for (i, child) in children.iter().enumerate() {
            // For i in [0..8], bits: x=1, y=2, z=4
            let offset_x = if (i & 1) != 0 { half } else { -half };
            let offset_y = if (i & 2) != 0 { half } else { -half };
            let offset_z = if (i & 4) != 0 { half } else { -half };

            let child_center = parent_center + Vec3::new(offset_x, offset_y, offset_z);

            // Draw the child bounding box
            gizmos.cuboid(
                Transform::from_translation(child_center).with_scale(Vec3::splat(child_size)),
                Color::rgba(0.5, 1.0, 0.5, 0.15), // greenish
            );

            // Recurse
            visualize_recursive_center(
                gizmos,
                child,
                child_center,
                child_size,
                depth + 1,
                max_depth,
            );
        }
    } else {
        // If node.is_leaf && node.voxel.is_some(), draw a smaller marker
        if node.is_leaf {
            if let Some(voxel) = node.voxel {
                // We'll choose a size that's a fraction of the parent's size.
                // For example, 25% of the parent bounding box dimension.
                let leaf_size = parent_size * 0.25;

                // Draw a small cuboid at the same center as the parent node.
                gizmos.cuboid(
                    Transform::from_translation(parent_center)
                        .with_scale(Vec3::splat(leaf_size)),
                    voxel.color,
                );
            }
        }
    }
}
fn selction_visualizer(mut gizmos: Gizmos,
                       camera_query: Query<&Selector, With<Camera>>,
                       octree_query: Query<(&SparseVoxelOctree, &Transform)>,){
    
    
    
    
    
}


#[allow(dead_code)]
pub fn draw_grid(
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<Camera>>,
    octree_query: Query<(&SparseVoxelOctree, &Transform)>,
) {
    let camera_tf = camera_query.single();
    let camera_pos = camera_tf.translation;

    for (octree, octree_tf) in octree_query.iter() {
        let half_size = octree.size * 0.5;
        let root_center = octree_tf.translation;

        // Voxel spacing at max depth
        let spacing = octree.get_spacing_at_depth(octree.max_depth);
        let grid_count = (octree.size / spacing) as i32;

        // We'll define the bounding region as [center-half_size .. center+half_size].
        // So the min corner is (root_center - half_size).
        let min_corner = root_center - Vec3::splat(half_size);

        // Draw lines in X & Z directions (like a ground plane).
        for i in 0..=grid_count {
            let offset = i as f32 * spacing;

            // 1) line along Z
            let x = min_corner.x + offset;
            let z1 = min_corner.z;
            let z2 = min_corner.z + (grid_count as f32 * spacing);

            let p1 = Vec3::new(x, min_corner.y, z1);
            let p2 = Vec3::new(x, min_corner.y, z2);

            // offset by -camera_pos for stable Gizmos in large coords
            let p1_f32 = p1 - camera_pos;
            let p2_f32 = p2 - camera_pos;
            gizmos.line(p1_f32, p2_f32, Color::WHITE);

            // 2) line along X
            let z = min_corner.z + offset;
            let x1 = min_corner.x;
            let x2 = min_corner.x + (grid_count as f32 * spacing);

            let p3 = Vec3::new(x1, min_corner.y, z) - camera_pos;
            let p4 = Vec3::new(x2, min_corner.y, z) - camera_pos;
            gizmos.line(p3, p4, Color::WHITE);
        }
    }
}