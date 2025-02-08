// Chunk Rendering

use bevy::math::{DQuat, DVec3};
use bevy::prelude::*;
use bevy::utils::info;
use bevy_asset::RenderAssetUsages;
use bevy_render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use crate::helper::large_transform::{DoubleTransform, WorldOffset};
use crate::systems::voxels::structure::{ChunkEntities, ChunkMarker, SparseVoxelOctree, CHUNK_BUILD_BUDGET, CHUNK_RENDER_DISTANCE, NEIGHBOR_OFFSETS};
use crate::helper::large_transform::get_true_world_position;
use crate::systems::voxels::helper::face_orientation;

/*pub fn render(
    mut commands: Commands,
    mut query: Query<&mut SparseVoxelOctree>,
    mut octree_transform_query: Query<&DoubleTransform, With<SparseVoxelOctree>>,
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
        if !octree.dirty_chunks.is_empty() {
            // Clear existing render objects
            for entity in render_object_query.iter() {
                commands.entity(entity).despawn();
            }

            // Collect the voxels to render
            let voxels = octree.traverse();

            let mut voxel_meshes = Vec::new();

            for (x, y, z, _color, depth) in voxels {
                let voxel_size = octree.get_spacing_at_depth(depth) as f32;

                // Calculate the world position of the voxel
                let world_position = Vec3::new(
                    (x * octree.size as f32) + (voxel_size / 2.0) - (octree.size / 2.0) as f32,
                    (y * octree.size as f32) + (voxel_size / 2.0) - (octree.size / 2.0) as f32,
                    (z * octree.size as f32) + (voxel_size / 2.0) - (octree.size / 2.0) as f32,
                );

                // Convert world_position components to f64 for neighbor checking
                let world_x = world_position.x as f64;
                let world_y = world_position.y as f64;
                let world_z = world_position.z as f64;

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
                    transform: Default::default(),
                    ..Default::default()
                },
                VoxelTerrainMarker {},
                DoubleTransform {
                    translation: octree_transform_query.single().translation,
                    rotation: DQuat::IDENTITY,
                    scale: DVec3::ONE,
                },
            ));

            // Reset the dirty flag once the update is complete
            octree.dirty_chunks.clear()
        }
    }
}
*/


#[derive(Component)]
pub struct VoxelTerrainMarker;


pub fn render(
    mut commands: Commands,
    mut octree_query: Query<&mut SparseVoxelOctree>,
    octree_transform_query: Query<&DoubleTransform, With<SparseVoxelOctree>>,
    mut chunk_entities: ResMut<ChunkEntities>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    // Use DoubleTransform for the camera
    camera_query: Query<&DoubleTransform, With<Camera>>,
) {
    let mut octree = match octree_query.get_single_mut() {
        Ok(o) => o,
        Err(_) => return,
    };

    let camera_dt = match camera_query.get_single() {
        Ok(dt) => dt,
        Err(_) => return,
    };
    // Convert camera's double position to f32 for distance calculations.
    let camera_pos = camera_dt.translation.as_vec3();

    let octree_dt = octree_transform_query.single();
    let octree_offset = octree_dt.translation.as_vec3();

    // Define chunk sizing.
    let step = octree.get_spacing_at_depth(octree.max_depth);
    let chunk_world_size = octree.get_chunk_size() as f32 * step as f32;

    // 1) DESPAWN out-of-range chunks.
    let mut chunks_to_remove = Vec::new();
    for (&(cx, cy, cz), &entity) in chunk_entities.map.iter() {
        let chunk_min = Vec3::new(
            cx as f32 * chunk_world_size,
            cy as f32 * chunk_world_size,
            cz as f32 * chunk_world_size,
        );
        let chunk_center = chunk_min + Vec3::splat(chunk_world_size * 0.5);
        let final_center = octree_offset + chunk_center;
        let dist = camera_pos.distance(final_center);
        if dist > CHUNK_RENDER_DISTANCE as f32 {
            chunks_to_remove.push((cx, cy, cz, entity));
        }
    }
    for (cx, cy, cz, e) in chunks_to_remove {
        commands.entity(e).despawn();
        chunk_entities.map.remove(&(cx, cy, cz));
    }

    // 2) LOAD new in-range chunks with nearest-first ordering.
    let camera_cx = ((camera_pos.x - octree_offset.x) / chunk_world_size).floor() as i32;
    let camera_cy = ((camera_pos.y - octree_offset.y) / chunk_world_size).floor() as i32;
    let camera_cz = ((camera_pos.z - octree_offset.z) / chunk_world_size).floor() as i32;

    let half_chunks = (CHUNK_RENDER_DISTANCE / chunk_world_size as f64).ceil() as i32;
    let mut new_chunks_to_spawn = Vec::new();
    for dx in -half_chunks..=half_chunks {
        for dy in -half_chunks..=half_chunks {
            for dz in -half_chunks..=half_chunks {
                let cc = (camera_cx + dx, camera_cy + dy, camera_cz + dz);
                if !chunk_entities.map.contains_key(&cc) {
                    let chunk_min = Vec3::new(
                        cc.0 as f32 * chunk_world_size,
                        cc.1 as f32 * chunk_world_size,
                        cc.2 as f32 * chunk_world_size,
                    );
                    let chunk_center = chunk_min + Vec3::splat(chunk_world_size * 0.5);
                    let final_center = octree_offset + chunk_center;
                    let dist = camera_pos.distance(final_center);
                    if dist <= CHUNK_RENDER_DISTANCE as f32 {
                        new_chunks_to_spawn.push(cc);
                    }
                }
            }
        }
    }
    // Sort candidate chunks by distance (nearest first).
    new_chunks_to_spawn.sort_by(|a, b| {
        let pos_a = octree_offset
            + Vec3::new(
            a.0 as f32 * chunk_world_size,
            a.1 as f32 * chunk_world_size,
            a.2 as f32 * chunk_world_size,
        )
            + Vec3::splat(chunk_world_size * 0.5);
        let pos_b = octree_offset
            + Vec3::new(
            b.0 as f32 * chunk_world_size,
            b.1 as f32 * chunk_world_size,
            b.2 as f32 * chunk_world_size,
        )
            + Vec3::splat(chunk_world_size * 0.5);
        camera_pos
            .distance(pos_a)
            .partial_cmp(&camera_pos.distance(pos_b))
            .unwrap()
    });

    let build_budget = 5; // Maximum chunks to build per frame.
    let mut spawn_count = 0;
    for cc in new_chunks_to_spawn {
        if spawn_count >= build_budget {
            break;
        }
        // Compute chunk's world position.
        let chunk_min = Vec3::new(
            cc.0 as f32 * chunk_world_size,
            cc.1 as f32 * chunk_world_size,
            cc.2 as f32 * chunk_world_size,
        );
        let chunk_center = chunk_min + Vec3::splat(chunk_world_size * 0.5);
        // Check if this chunk has any voxels.
        if let Some(chunk_node) =
            octree.get_chunk_node(chunk_center.x as f64, chunk_center.y as f64, chunk_center.z as f64)
        {
            if octree.has_volume(chunk_node) {
                info!("Loading chunk at: {},{},{} (has volume)", cc.0, cc.1, cc.2);
            }
        }
        build_and_spawn_chunk(
            &mut commands,
            &octree,
            &mut meshes,
            &mut materials,
            &mut chunk_entities,
            cc,
            octree_offset,
        );
        spawn_count += 1;
    }

    // 3) Rebuild dirty chunks (if any) with nearest-first ordering and budget.
    if !octree.dirty_chunks.is_empty() {
        let mut dirty = octree.dirty_chunks.drain().collect::<Vec<_>>();
        dirty.sort_by(|a, b| {
            let pos_a = octree_offset
                + Vec3::new(
                a.0 as f32 * chunk_world_size,
                a.1 as f32 * chunk_world_size,
                a.2 as f32 * chunk_world_size,
            )
                + Vec3::splat(chunk_world_size * 0.5);
            let pos_b = octree_offset
                + Vec3::new(
                b.0 as f32 * chunk_world_size,
                b.1 as f32 * chunk_world_size,
                b.2 as f32 * chunk_world_size,
            )
                + Vec3::splat(chunk_world_size * 0.5);
            camera_pos
                .distance(pos_a)
                .partial_cmp(&camera_pos.distance(pos_b))
                .unwrap()
        });

        let mut rebuild_count = 0;
        for chunk_coord in dirty {
            if rebuild_count >= build_budget {
                octree.dirty_chunks.insert(chunk_coord);
                continue;
            }
            let chunk_min = Vec3::new(
                chunk_coord.0 as f32 * chunk_world_size,
                chunk_coord.1 as f32 * chunk_world_size,
                chunk_coord.2 as f32 * chunk_world_size,
            );
            let chunk_center = chunk_min + Vec3::splat(chunk_world_size * 0.5);
            let final_center = octree_offset + chunk_center;
            let dist = camera_pos.distance(final_center);

            if dist <= CHUNK_RENDER_DISTANCE as f32 {
                if let Some(chunk_node) =
                    octree.get_chunk_node(chunk_center.x as f64, chunk_center.y as f64, chunk_center.z as f64)
                {
                    if octree.has_volume(chunk_node) {
                        info!(
                            "Rebuilding chunk at: {},{},{} (has volume)",
                            chunk_coord.0, chunk_coord.1, chunk_coord.2
                        );
                    }
                }
                if let Some(e) = chunk_entities.map.remove(&chunk_coord) {
                    commands.entity(e).despawn();
                }
                build_and_spawn_chunk(
                    &mut commands,
                    &octree,
                    &mut meshes,
                    &mut materials,
                    &mut chunk_entities,
                    chunk_coord,
                    octree_offset,
                );
                rebuild_count += 1;
            } else {
                if let Some(e) = chunk_entities.map.remove(&chunk_coord) {
                    commands.entity(e).despawn();
                }
            }
        }
    }
}



fn build_and_spawn_chunk(
    commands: &mut Commands,
    octree: &SparseVoxelOctree,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_entities: &mut ChunkEntities,
    chunk_coord: (i32, i32, i32),
    octree_offset: Vec3,
) {
    let face_meshes = build_chunk_geometry(octree, chunk_coord);
    if face_meshes.is_empty() {
        return;
    }

    let merged = merge_meshes(face_meshes);
    let mesh_handle = meshes.add(merged);

    let step = octree.get_spacing_at_depth(octree.max_depth);
    let chunk_world_size = octree.get_chunk_size() as f64 * step;
    let chunk_min = Vec3::new(
        chunk_coord.0 as f32 * chunk_world_size as f32,
        chunk_coord.1 as f32 * chunk_world_size as f32,
        chunk_coord.2 as f32 * chunk_world_size as f32,
    );
    let final_pos = octree_offset + chunk_min;

    let e = commands.spawn((
        PbrBundle {
            mesh: mesh_handle.into(),
            material: materials.add(StandardMaterial {
                base_color: Color::rgb(0.8, 0.7, 0.6),
                ..default()
            }).into(),
            transform: Transform::from_translation(final_pos),
            ..default()
        },
        VoxelTerrainMarker,
        DoubleTransform {
            translation: DVec3::from(final_pos),
            rotation: DQuat::IDENTITY,
            scale: DVec3::ONE,
        },
    ))
        .id();

    chunk_entities.map.insert(chunk_coord, e);
}

fn build_chunk_geometry(
    octree: &SparseVoxelOctree,
    (cx, cy, cz): (i32, i32, i32),
) -> Vec<Mesh> {
    let mut face_meshes = Vec::new();

    // step in world units for one voxel at max_depth
    let step = octree.get_spacing_at_depth(octree.max_depth);
    let chunk_size = octree.get_chunk_size();

    // chunk is 16 voxels => chunk_min in world space:
    let chunk_min_x = cx as f64 * (chunk_size as f64 * step);
    let chunk_min_y = cy as f64 * (chunk_size as f64 * step);
    let chunk_min_z = cz as f64 * (chunk_size as f64 * step);

    // for local offset
    let chunk_min_f32 = Vec3::new(
        chunk_min_x as f32,
        chunk_min_y as f32,
        chunk_min_z as f32,
    );
    let voxel_size_f = step as f32;

    // i in [0..16] => corner is chunk_min_x + i*step
    // no +0.5 => corners approach
    for i in 0..chunk_size {
        let vx = chunk_min_x + i as f64 * step;
        for j in 0..chunk_size {
            let vy = chunk_min_y + j as f64 * step;
            for k in 0..chunk_size {
                let vz = chunk_min_z + k as f64 * step;

                // check if we have a voxel at that corner
                if let Some(_) = octree.get_voxel_at_world_coords(vx, vy, vz) {
                    // check neighbors
                    for &(dx, dy, dz) in NEIGHBOR_OFFSETS.iter() {
                        let nx = vx + dx as f64 * step;
                        let ny = vy + dy as f64 * step;
                        let nz = vz + dz as f64 * step;

                        if octree.get_voxel_at_world_coords(nx, ny, nz).is_none() {
                            let (normal, local_offset) = crate::systems::voxels::helper::face_orientation(dx, dy, dz, voxel_size_f);

                            // The voxel corner in chunk-local coords
                            let voxel_corner_local = Vec3::new(vx as f32, vy as f32, vz as f32)
                                - chunk_min_f32;

                            // generate face
                            // e.g. center might be the corner + 0.5 offset, or
                            // we can just treat the corner as the "center" in your face calc
                            // but let's do it carefully:
                            let face_center_local = voxel_corner_local + Vec3::splat(voxel_size_f*0.5);

                            let face_mesh = generate_face(
                                normal,
                                local_offset,
                                face_center_local,
                                voxel_size_f / 2.0,
                            );
                            face_meshes.push(face_mesh);
                        }
                    }
                }
            }
        }
    }
    face_meshes
}



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

