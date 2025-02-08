use bevy::color::palettes::basic::{BLACK, RED, YELLOW};
use bevy::color::palettes::css::GREEN;
use bevy::math::{DQuat, DVec3, Vec3};
use bevy::pbr::wireframe::Wireframe;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use bevy_egui::egui::emath::Numeric;
use bevy_render::prelude::*;
use crate::helper::large_transform::DoubleTransform;
use crate::systems::voxels::structure::{ChunkEntities, OctreeNode, SparseVoxelOctree};

pub fn visualize_octree(
    mut gizmos: Gizmos,
    camera_query: Query<&DoubleTransform, With<Camera>>,
    octree_query: Query<(&SparseVoxelOctree, &DoubleTransform)>,
) {
    let camera_tf = camera_query.single(); // your "real" camera position in double precision
    let camera_pos = camera_tf.translation; // DVec3

    for (octree, octree_tf) in octree_query.iter() {
        let octree_world_pos = octree_tf.translation;
        visualize_recursive(
            &mut gizmos,
            &octree.root,
            octree_world_pos,               // octree’s root center
            octree.size,
            octree.max_depth,
            camera_pos,
        );
    }
}

fn visualize_recursive(
    gizmos: &mut Gizmos,
    node: &OctreeNode,
    node_center: DVec3,
    node_size: f64,
    depth: u32,
    camera_pos: DVec3,
) {
    if depth == 0 {
        return;
    }

    // If you want to draw the bounding box of this node:
    /*let half = node_size as f32 * 0.5;*/
    // Convert double center -> local f32 position
    let center_f32 = (node_center - camera_pos).as_vec3();

    // A quick approach: draw a wireframe cube by drawing lines for each edge
    // Or use "cuboid gizmo" methods in future bevy versions that might exist.
    /*draw_wire_cube(gizmos, center_f32, half, Color::YELLOW);*/

    gizmos.cuboid(
        Transform::from_translation(center_f32).with_scale(Vec3::splat(node_size as f32)),
        BLACK,
    );

    // Recurse children
    if let Some(children) = &node.children {
        let child_size = node_size / 2.0;
        for (i, child) in children.iter().enumerate() {
            let offset_x = if (i & 1) == 1 { child_size / 2.0 } else { -child_size / 2.0 };
            let offset_y = if (i & 2) == 2 { child_size / 2.0 } else { -child_size / 2.0 };
            let offset_z = if (i & 4) == 4 { child_size / 2.0 } else { -child_size / 2.0 };

            let child_center = DVec3::new(
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
    camera_query: Query<&DoubleTransform, With<Camera>>,
    octree_query: Query<(&SparseVoxelOctree, &DoubleTransform)>,
) {
    // 1) Get the camera’s double transform for offset
    let camera_tf = camera_query.single();
    let camera_pos = camera_tf.translation; // DVec3

    for (octree, octree_dtf) in octree_query.iter() {


        // 2) Octree’s double position
        let octree_pos = octree_dtf.translation; // e.g. [100_000, 0, 0] in double space

        // 3) Compute spacing in f64
        let grid_spacing = octree.get_spacing_at_depth(octree.max_depth) as f64;
        let grid_size = (octree.size / grid_spacing) as i32;

        // 4) Start position in local "octree space"
        //    We'll define the bounding region from [-size/2, +size/2]
        let half_size = octree.size * 0.5;
        let start_position = -half_size; // f64

        // 5) Loop over lines
        for i in 0..=grid_size {
            // i-th line offset
            let offset = i as f64 * grid_spacing;

            // a) Lines along Z
            //    from (start_position + offset, 0, start_position)
            //    to   (start_position + offset, 0, start_position + grid_size * spacing)
            {
                let x = start_position + offset;
                let z1 = start_position;
                let z2 = start_position + (grid_size as f64 * grid_spacing);

                // Convert these points to "world double" by adding octree_pos
                let p1_d = DVec3::new(x, 0.0, z1) + octree_pos;
                let p2_d = DVec3::new(x, 0.0, z2) + octree_pos;

                // Then offset by camera_pos, convert to f32
                let p1_f32 = (p1_d - camera_pos).as_vec3();
                let p2_f32 = (p2_d - camera_pos).as_vec3();

                // Draw the line
                gizmos.line(p1_f32, p2_f32, Color::WHITE);
            }

            // b) Lines along X
            //    from (start_position, 0, start_position + offset)
            //    to   (start_position + grid_size * spacing, 0, start_position + offset)
            {
                let z = start_position + offset;
                let x1 = start_position;
                let x2 = start_position + (grid_size as f64 * grid_spacing);

                let p1_d = DVec3::new(x1, 0.0, z) + octree_pos;
                let p2_d = DVec3::new(x2, 0.0, z) + octree_pos;

                let p1_f32 = (p1_d - camera_pos).as_vec3();
                let p2_f32 = (p2_d - camera_pos).as_vec3();

                gizmos.line(p1_f32, p2_f32, Color::WHITE);
            }
        }
    }
}

/*#[derive(Component)]
pub struct GridMarker;

pub fn draw_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(Entity, &SparseVoxelOctree)>, // Query to access the octree
    grid_query: Query<Entity, With<GridMarker>>, // Query to find existing grid entities
) {
    for (_, octree) in query.iter() {
        if octree.show_world_grid {
            // If grid should be shown, check if it already exists
            if grid_query.iter().next().is_none() {
                // Grid doesn't exist, so create it
                let grid_spacing = octree.get_spacing_at_depth(octree.max_depth) as f32; // Get spacing at the specified depth
                let grid_size = (octree.size / grid_spacing as f64) as i32; // Determine the number of lines needed

                let mut positions = Vec::new();
                let mut indices = Vec::new();

                // Calculate the start position to center the grid
                let start_position = -(octree.size as f32 / 2.0);

                // Create lines along the X and Z axes based on calculated spacing
                for i in 0..=grid_size {
                    // Lines along the Z-axis
                    positions.push([start_position + i as f32 * grid_spacing, 0.0, start_position]);
                    positions.push([start_position + i as f32 * grid_spacing, 0.0, start_position + grid_size as f32 * grid_spacing]);

                    // Indices for the Z-axis lines
                    let base_index = (i * 2) as u32;
                    indices.push(base_index);
                    indices.push(base_index + 1);

                    // Lines along the X-axis
                    positions.push([start_position, 0.0, start_position + i as f32 * grid_spacing]);
                    positions.push([start_position + grid_size as f32 * grid_spacing, 0.0, start_position + i as f32 * grid_spacing]);

                    // Indices for the X-axis lines
                    let base_index_x = ((grid_size + 1 + i) * 2) as u32;
                    indices.push(base_index_x);
                    indices.push(base_index_x + 1);
                }

                // Create the line mesh
                let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
                mesh.insert_indices(Indices::U32(indices));

                
                let color = bevy::color::Color::srgba(204.0 / 255.0, 0.0, 218.0 / 255.0, 15.0 / 255.0);
                

                // Spawn the entity with the line mesh
                commands.spawn(PbrBundle {
                    mesh: meshes.add(mesh).into(),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        unlit: true, // Makes the lines visible without lighting
                        ..Default::default()
                    }).into(),
                    transform: Transform::default(),
                    ..Default::default()
                })
                    .insert(GridMarker); // Add a marker component to identify the grid
            }
        } else {
            // If grid should not be shown, remove any existing grid
            for grid_entity in grid_query.iter() {
                commands.entity(grid_entity).despawn();
            }
        }
    }
}
*/

/*#[derive(Component)]
pub struct BuildVisualization;

#[derive(Debug)]
pub struct EphemeralLine {
    pub start: Vec3,
    pub end: Vec3,
    pub color: Color,
    pub time_left: f32, // in seconds
}

#[derive(Resource, Default)]
pub struct EphemeralLines {
    pub lines: Vec<EphemeralLine>,
}

pub fn ephemeral_lines_system(
    mut lines: ResMut<EphemeralLines>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    // Retain only those with time_left > 0, and while they're active, draw them
    lines.lines.retain_mut(|line| {
        line.time_left -= dt;
        if line.time_left > 0.0 {
            // Draw the line with gizmos
            gizmos.line(line.start, line.end, line.color);
            // Keep it
            true
        } else {
            // Time’s up, discard
            false
        }
    });
}*/

// System that draws wireframe boxes around each chunk's bounding region.
pub fn debug_draw_chunks_system(
    chunk_entities: Res<ChunkEntities>,

    // If your chunk placement depends on the octree's transform
    // query that. Otherwise you can skip if they're always at (0,0,0).
    octree_query: Query<(&SparseVoxelOctree, &DoubleTransform)>,
    // Optional: If you want large-world offset for camera, we can subtract camera position.
    // If you don't have floating-origin logic, you can skip this.
    camera_query: Query<&DoubleTransform, With<Camera>>,

    mut gizmos: Gizmos,
) {
    // We'll get the octree transform offset if we have only one octree.
    // Adjust if you have multiple.
    let (octree, octree_tf) = match octree_query.get_single() {
        Ok(x) => x,
        Err(_) => return,
    };

    // 1) Determine the world size of a single voxel
    let step = octree.get_spacing_at_depth(octree.max_depth);
    // chunk_size in world units = 16 voxels * step
    let chunk_size_world = octree.get_chunk_size() as f64 * step;

    // 2) We'll also get the octree's offset in double precision
    let octree_pos_d = octree_tf.translation;

    // If you want a floating origin approach, subtract the camera's double position:
    let camera_tf = match camera_query.get_single() {
        Ok(tf) => tf,
        Err(_) => return,
    };
    let camera_pos_d = camera_tf.translation;

    // For each chunk coordinate
    for (&(cx, cy, cz), _entity) in chunk_entities.map.iter() {
        // 4) Chunk bounding box in double precision
        let chunk_min_d = octree_pos_d
            + DVec3::new(
            cx as f64 * chunk_size_world,
            cy as f64 * chunk_size_world,
            cz as f64 * chunk_size_world,
        );
        let chunk_max_d = chunk_min_d + DVec3::splat(chunk_size_world);

        // 5) Convert to local f32 near the camera
        let min_f32 = (chunk_min_d - camera_pos_d).as_vec3();
        let max_f32 = (chunk_max_d - camera_pos_d).as_vec3();

        // 6) Draw ephemeral lines for the box
        draw_wire_cube(&mut gizmos, min_f32, max_f32, Color::from(YELLOW));
    }
}

/// Helper function to draw a wireframe box from `min` to `max` in ephemeral gizmos.
fn draw_wire_cube(
    gizmos: &mut Gizmos,
    min: Vec3,
    max: Vec3,
    color: Color,
) {
    // corners
    let c0 = Vec3::new(min.x, min.y, min.z);
    let c1 = Vec3::new(max.x, min.y, min.z);
    let c2 = Vec3::new(min.x, max.y, min.z);
    let c3 = Vec3::new(max.x, max.y, min.z);
    let c4 = Vec3::new(min.x, min.y, max.z);
    let c5 = Vec3::new(max.x, min.y, max.z);
    let c6 = Vec3::new(min.x, max.y, max.z);
    let c7 = Vec3::new(max.x, max.y, max.z);

    // edges
    // bottom face
    gizmos.line(c0, c1, color);
    gizmos.line(c1, c3, color);
    gizmos.line(c3, c2, color);
    gizmos.line(c2, c0, color);
    // top face
    gizmos.line(c4, c5, color);
    gizmos.line(c5, c7, color);
    gizmos.line(c7, c6, color);
    gizmos.line(c6, c4, color);
    // verticals
    gizmos.line(c0, c4, color);
    gizmos.line(c1, c5, color);
    gizmos.line(c2, c6, color);
    gizmos.line(c3, c7, color);
}