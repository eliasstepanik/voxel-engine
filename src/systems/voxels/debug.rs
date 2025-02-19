use bevy::color::palettes::basic::{BLACK, RED, YELLOW};
use bevy::color::palettes::css::GREEN;
use bevy::math::{DQuat, Vec3};
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy_egui::egui::emath::Numeric; 
use crate::systems::voxels::structure::{ OctreeNode, SparseVoxelOctree};

pub fn visualize_octree(
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<Camera>>,
    octree_query: Query<(&SparseVoxelOctree, &Transform)>,
) {
    let camera_tf = camera_query.single(); // your "real" camera position in double precision
    let camera_pos = camera_tf.translation; // DVec3

    for (octree, octree_tf) in octree_query.iter() {
        visualize_recursive(
            &mut gizmos,
            &octree.root,
            octree_tf.translation,               // octree’s root center
            octree.size,
            octree.max_depth,
            camera_pos,
        );
    }
}

fn visualize_recursive(
    gizmos: &mut Gizmos,
    node: &OctreeNode,
    node_center: Vec3,
    node_size: f32,
    depth: u32,
    camera_pos: Vec3,
) {
    if depth == 0 {
        return;
    }

    // If you want to draw the bounding box of this node:
    /*let half = node_size as f32 * 0.5;*/
    // Convert double center -> local f32 position
    let center_f32 = (node_center - camera_pos);

    // A quick approach: draw a wireframe cube by drawing lines for each edge
    // Or use "cuboid gizmo" methods in future bevy versions that might exist.
    /*draw_wire_cube(gizmos, center_f32, half, Color::YELLOW);*/


    
    gizmos.cuboid(
        Transform::from_translation(center_f32).with_scale(Vec3::splat(node_size)),
        BLACK,
    );

    // Recurse children
    if let Some(children) = &node.children {
        let child_size = node_size / 2.0;


        for (i, child) in children.iter().enumerate() {
            let offset_x = if (i & 1) == 1 { child_size / 2.0 } else { -child_size / 2.0 };
            let offset_y = if (i & 2) == 2 { child_size / 2.0 } else { -child_size / 2.0 };
            let offset_z = if (i & 4) == 4 { child_size / 2.0 } else { -child_size / 2.0 };

            let child_center = Vec3::new(
                node_center.x + offset_x,
                node_center.y + offset_y,
                node_center.z + offset_z,
            );

            visualize_recursive(
                gizmos,
                child,
                child_center,
                child_size,
                depth - 1,
                camera_pos,
            );
        }
    }
}


#[allow(dead_code)]
pub fn draw_grid(
    mut gizmos: Gizmos,
    camera_query: Query<&Transform, With<Camera>>,
    octree_query: Query<(&SparseVoxelOctree, &Transform)>,
) {
    // 1) Get the camera’s double transform for offset
    let camera_tf = camera_query.single();
    let camera_pos = camera_tf.translation; // DVec3

    for (octree, octree_dtf) in octree_query.iter() {


        // 2) Octree’s double position
        let octree_pos = octree_dtf.translation; // e.g. [100_000, 0, 0] in double space

        // 3) Compute spacing in f32
        let grid_spacing = octree.get_spacing_at_depth(octree.max_depth) as f32;
        let grid_size = (octree.size / grid_spacing) as i32;

        // 4) Start position in local "octree space"
        //    We'll define the bounding region from [-size/2, +size/2]
        let half_size = octree.size * 0.5;
        let start_position = -half_size; // f32

        // 5) Loop over lines
        for i in 0..=grid_size {
            // i-th line offset
            let offset = i as f32 * grid_spacing;

            // a) Lines along Z
            //    from (start_position + offset, 0, start_position)
            //    to   (start_position + offset, 0, start_position + grid_size * spacing)
            {
                let x = start_position + offset;
                let z1 = start_position;
                let z2 = start_position + (grid_size as f32 * grid_spacing);

                // Convert these points to "world double" by adding octree_pos
                let p1_d = Vec3::new(x, 0.0, z1) + octree_pos;
                let p2_d = Vec3::new(x, 0.0, z2) + octree_pos;

                // Then offset by camera_pos, convert to f32
                let p1_f32 = (p1_d - camera_pos);
                let p2_f32 = (p2_d - camera_pos);

                // Draw the line
                gizmos.line(p1_f32, p2_f32, Color::WHITE);
            }

            // b) Lines along X
            //    from (start_position, 0, start_position + offset)
            //    to   (start_position + grid_size * spacing, 0, start_position + offset)
            {
                let z = start_position + offset;
                let x1 = start_position;
                let x2 = start_position + (grid_size as f32 * grid_spacing);

                let p1_d = Vec3::new(x1, 0.0, z) + octree_pos;
                let p2_d = Vec3::new(x2, 0.0, z) + octree_pos;

                let p1_f32 = (p1_d - camera_pos);
                let p2_f32 = (p2_d - camera_pos);

                gizmos.line(p1_f32, p2_f32, Color::WHITE);
            }
        }
    }
}
