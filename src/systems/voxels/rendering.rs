use std::collections::HashMap;
use bevy::color::palettes::basic::BLUE;
use bevy::prelude::*;
use bevy::utils::info;
use bevy_asset::RenderAssetUsages;
use bevy_render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy_render::render_resource::Face;
use log::info;
use crate::systems::ui_system::SpeedDisplay;
use crate::systems::voxels::octree;
use crate::systems::voxels::structure::{SparseVoxelOctree, NEIGHBOR_OFFSETS};

#[derive(Component)]
pub struct VoxelTerrainMarker {}


pub fn render(
    mut commands: Commands,
    mut query: Query<&mut SparseVoxelOctree>,
    render_object_query: Query<Entity, With<VoxelTerrainMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {


    for mut octree in query.iter_mut() {
        // Only update when marked dirty
        if !octree.dirty.is_empty() {
            // Remove old render objects
            for entity in render_object_query.iter() {
                commands.entity(entity).despawn();
            }

            // Get the voxel centers (world positions), color, and depth.
            let voxels = octree.traverse();

            // Debug: Log the number of voxels traversed.
            info!("Voxel count: {}", voxels.len());

            let mut voxel_meshes = Vec::new();

            for (world_position, _color, depth) in voxels {
                // Get the size of the voxel at the current depth.
                let voxel_size = octree.get_spacing_at_depth(depth);

                // The traverse method already returns the voxel center in world space.

                // For each neighbor direction, check if this voxel face is exposed.
                for &(dx, dy, dz) in NEIGHBOR_OFFSETS.iter() {
                    // Pass the world-space voxel center directly.
                    if !octree.has_neighbor(world_position, dx as i32, dy as i32, dz as i32, depth) {

                        // Determine face normal and the local offset for the face.
                        let (normal, offset) = match (dx, dy, dz) {
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

                        voxel_meshes.push(generate_face(
                            world_position + offset, // offset the face
                            voxel_size / 2.0,
                            normal
                        ));
                    }
                }
            }

            // Merge all the face meshes into a single mesh.
            let mesh = merge_meshes(voxel_meshes);
            let cube_handle = meshes.add(mesh);

            // Create a material with cull_mode disabled to see both sides (for debugging)
            let material = materials.add(StandardMaterial {
                base_color: Color::srgba(0.8, 0.7, 0.6, 1.0),
                cull_mode: Some(Face::Back), // disable culling for debugging
                ..Default::default()
            });


            // Spawn the mesh into the scene
            commands.spawn((
                PbrBundle {
                    mesh: Mesh3d::from(cube_handle),
                    material: MeshMaterial3d::from(material),
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                    ..Default::default()
                },
                VoxelTerrainMarker {},
            ));

            // Reset the dirty flag after updating.
            octree.dirty.clear();
        }
    }
}

fn generate_face(position: Vec3, face_size: f32, normal: Vec3) -> Mesh {
    // Initialize an empty mesh with triangle topology
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());

    // Define a quad centered at the origin
    let mut positions = vec![
        [-face_size, -face_size, 0.0],
        [ face_size, -face_size, 0.0],
        [ face_size,  face_size, 0.0],
        [-face_size,  face_size, 0.0],
    ];

    // Normalize the provided normal to ensure correct rotation
    let normal = normal.normalize();
    // Compute a rotation that aligns the default +Z with the provided normal
    let rotation = Quat::from_rotation_arc(Vec3::Z, normal);

    // Rotate and translate the vertices based on the computed rotation and provided position
    for p in positions.iter_mut() {
        let vertex = rotation * Vec3::from(*p) + position;
        *p = [vertex.x, vertex.y, vertex.z];
    }

    let uvs = vec![
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
    ];

    let indices = Indices::U32(vec![0, 1, 2, 2, 3, 0]);

    // Use the provided normal for all vertices
    let normals = vec![[normal.x, normal.y, normal.z]; 4];

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