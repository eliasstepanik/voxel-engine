use std::collections::HashMap;
use bevy::color::palettes::basic::BLUE;
use bevy::prelude::*;
use bevy_asset::RenderAssetUsages;
use bevy_render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use crate::systems::ui_system::SpeedDisplay;
use crate::systems::voxels::structure::{SparseVoxelOctree, NEIGHBOR_OFFSETS};



pub fn render(
    mut commands: Commands,
    mut query: Query<&mut SparseVoxelOctree>,
    octree_transform_query: Query<&Transform, With<SparseVoxelOctree>>,
    render_object_query: Query<Entity, With<VoxelTerrainMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    // Get the camera's current position (if needed for LOD calculations)
    let camera_transform = camera_query.single();
    let _camera_position = camera_transform.translation;


    for mut octree in query.iter_mut() {



        // Handle updates to the octree only if it is marked as dirty
        if octree.dirty {
            // Clear existing render objects
            for entity in render_object_query.iter() {
                commands.entity(entity).despawn();
            }

            // Collect the voxels to render
            let voxels = octree.traverse();

            let mut voxel_meshes = Vec::new();

            for (x, y, z, _color, depth) in voxels {
                let voxel_size = octree.get_spacing_at_depth(depth);

                // Calculate the world position of the voxel
                let world_position = Vec3::new(
                    (x * octree.size) + (voxel_size / 2.0) - (octree.size / 2.0),
                    (y * octree.size) + (voxel_size / 2.0) - (octree.size / 2.0),
                    (z * octree.size) + (voxel_size / 2.0) - (octree.size / 2.0),
                );

                // Convert world_position components to f32 for neighbor checking
                let world_x = world_position.x;
                let world_y = world_position.y;
                let world_z = world_position.z;

                // Iterate over all possible neighbor offsets
                for &(dx, dy, dz) in NEIGHBOR_OFFSETS.iter() {

                    // Check if there's no neighbor in this direction
                    if !octree.has_neighbor(world_x, world_y, world_z, dx as i32, dy as i32, dz as i32, depth) {

                        // Determine the face normal and local position based on the direction
                        let (normal, local_position) = match (dx, dy, dz) {
                            (-1.0, 0.0, 0.0) => (
                                Vec3::new(-1.0, 0.0, 0.0),
                                Vec3::new(-voxel_size / 2.0, 0.0, 0.0),
                            ),
                            (1.0, 0.0, 0.0) => (
                                Vec3::new(1.0, 0.0, 0.0),
                                Vec3::new(voxel_size / 2.0, 0.0, 0.0),
                            ),
                            (0.0, -1.0, 0.0) => (
                                Vec3::new(0.0, -1.0, 0.0),
                                Vec3::new(0.0, -voxel_size / 2.0, 0.0),
                            ),
                            (0.0, 1.0, 0.0) => (
                                Vec3::new(0.0, 1.0, 0.0),
                                Vec3::new(0.0, voxel_size / 2.0, 0.0),
                            ),
                            (0.0, 0.0, -1.0) => (
                                Vec3::new(0.0, 0.0, -1.0),
                                Vec3::new(0.0, 0.0, -voxel_size / 2.0),
                            ),
                            (0.0, 0.0, 1.0) => (
                                Vec3::new(0.0, 0.0, 1.0),
                                Vec3::new(0.0, 0.0, voxel_size / 2.0),
                            ),
                            _ => continue,
                        };

                        // Generate the face for rendering
                        voxel_meshes.push(generate_face(
                            normal,
                            local_position,
                            world_position,
                            voxel_size / 2.0,
                        ));
                    }
                }
            }

            // Merge the voxel meshes into a single mesh
            let mesh = merge_meshes(voxel_meshes);
            let cube_handle = meshes.add(mesh);

            // Spawn the mesh into the scene
            commands.spawn((
                PbrBundle {
                    mesh: Mesh3d::from(cube_handle),
                    material: MeshMaterial3d::from(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.8, 0.7, 0.6),
                        ..Default::default()
                    })),
                    transform: *octree_transform_query.single(),
                    ..Default::default()
                },
                VoxelTerrainMarker {},
            ));

            // Reset the dirty flag once the update is complete
            octree.dirty = false;
        }
    }
}



#[derive(Component)]
pub struct VoxelTerrainMarker;


fn generate_face(orientation: Vec3, local_position: Vec3, position: Vec3, face_size: f32) -> Mesh {
    // Initialize an empty mesh with triangle topology
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());


    let mut positions = vec![
        [-face_size, -face_size, 0.0],
        [face_size, -face_size, 0.0],
        [face_size, face_size, 0.0],
        [-face_size, face_size, 0.0],
    ];

    let rotation = Quat::from_rotation_arc(Vec3::Z, orientation);

    // Rotate and translate the vertices based on orientation and position
    for p in positions.iter_mut() {
        let vertex = rotation * Vec3::from(*p);
        let vertex = vertex + local_position + position; // Apply local and global translation
        *p = [vertex.x, vertex.y, vertex.z];
    }

    let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];

    let indices = Indices::U32(vec![0, 1, 2, 2, 3, 0]);

    let normal = rotation * Vec3::Z; // Since face is aligned to Vec3::Z initially
    let normals = vec![
        [normal.x, normal.y, normal.z]; // Use the same normal for all vertices
        4 // Four vertices in a quad
    ];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(indices);

    mesh
}
fn merge_meshes(meshes: Vec<Mesh>) -> Mesh {
    let mut merged_positions = Vec::new();
    let mut merged_uvs = Vec::new();
    let mut merged_normals = Vec::new(); // To store merged normals
    let mut merged_indices = Vec::new();

    for mesh in meshes {
        if let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
            let start_index = merged_positions.len();
            merged_positions.extend_from_slice(positions);

            // Extract UVs
            if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                merged_uvs.extend_from_slice(uvs);
            }

            // Extract normals
            if let Some(VertexAttributeValues::Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
                merged_normals.extend_from_slice(normals);
            }

            // Extract indices and apply offset
            if let Some(indices) = mesh.indices() {
                if let Indices::U32(indices) = indices {
                    let offset_indices: Vec<u32> = indices.iter().map(|i| i + start_index as u32).collect();
                    merged_indices.extend(offset_indices);
                }
            }
        }
    }

    // Create new merged mesh
    let mut merged_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

    // Insert attributes into the merged mesh
    merged_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, merged_positions);
    merged_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, merged_uvs);
    merged_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, merged_normals); // Insert merged normals
    merged_mesh.insert_indices(Indices::U32(merged_indices));

    merged_mesh
}

