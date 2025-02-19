use bevy::color::palettes::basic::*;
use bevy::color::palettes::css::{BEIGE, MIDNIGHT_BLUE, ORANGE, ORANGE_RED, SEA_GREEN};
use bevy::math::*;
use bevy::prelude::*;
use crate::systems::voxels::structure::{SparseVoxelOctree, Voxel};
/*pub fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
){
    // 1) Circular base
    commands.spawn((
        // Double precision
        DoubleTransform {
            translation: DVec3::new(0.0, 0.0, 10.0),
            // rotate -90 degrees around X so the circle is on the XY plane
            rotation: DQuat::from_euler(EulerRot::XYZ, -std::f32::consts::FRAC_PI_2, 0.0, 0.0),
            scale: DVec3::ONE,
        },
        // Bevy's transform components
        Transform::default(),
        GlobalTransform::default(),
        // 3D mesh + material
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
    ));

    // 2) Cube
    commands.spawn((
        // Double precision
        DoubleTransform {
            translation: DVec3::new(0.0, 0.5, 10.0),
            rotation: DQuat::IDENTITY,
            scale: DVec3::ONE,
        },
        // Bevy's transform components
        Transform::default(),
        GlobalTransform::default(),
        // 3D mesh + material
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::rgb_u8(124, 144, 255))),
    ));

    // 3) Point light
    commands.spawn((
        DoubleTransform {
            translation: DVec3::new(4.0, 8.0, 14.0),
            rotation: DQuat::IDENTITY,
            scale: DVec3::ONE,
        },
        Transform::default(),
        GlobalTransform::default(),
        PointLight {
            shadows_enabled: true,
            ..default()
        },
    ));
}
*/


pub fn setup(mut commands: Commands,) {


    let voxels_per_unit = 16;
    let unit_size = 1.0; // 1 unit in your coordinate space
    let voxel_size = unit_size / voxels_per_unit as f32;
    
    /*//Octree
    let octree_base_size = 64.0;
    let octree_depth = 10;*/

    // Octree parameters
    let octree_base_size = 64.0 * unit_size; // Octree's total size in your world space
    let octree_depth = 10;


    let mut octree = SparseVoxelOctree::new(octree_depth, octree_base_size as f32, false, false, false);

    
    let color = Color::rgb(0.2, 0.8, 0.2);
    /*generate_voxel_rect(&mut octree,color);*/
    /*generate_voxel_sphere(&mut octree, 10.0, color);*/

    generate_large_plane(&mut octree, 200, 200,color );
    
    /*octree.insert(0.0,0.0,0.0, Voxel::new(Color::from(RED)));*/

    
    commands.spawn(
        (
            Transform::default(),
            octree
        )
    );


    commands.spawn((
        Transform::default(),
        GlobalTransform::default(),
        PointLight {
            shadows_enabled: true,
            ..default()
        },
    ));

    // Insert the octree into the ECS
}



/// Naïve function to generate a spherical planet in the voxel octree.
/// - `planet_radius`: radius of the "planet" in your world-space units
/// - `voxel_step`: how finely to sample the sphere in the x/y/z loops
fn generate_voxel_sphere(
    octree: &mut SparseVoxelOctree,
    planet_radius: i32,
    voxel_color: Color,
) {
    // For simplicity, we center the sphere around (0,0,0).
    // We'll loop over a cubic region [-planet_radius, +planet_radius] in x, y, z
    let min = -planet_radius;
    let max = planet_radius;

    let step = octree.get_spacing_at_depth(octree.max_depth);

    for ix in min..=max {
        let x = ix;
        for iy in min..=max {
            let y = iy;
            for iz in min..=max {
                let z = iz;

                // Check if within sphere of radius `planet_radius`
                let dist2 = x * x + y * y + z * z;
                if dist2 <= planet_radius * planet_radius {
                    // Convert (x,y,z) to world space, stepping by `voxel_step`.
                    let wx = x as f32 * step;
                    let wy = y as f32 * step;
                    let wz = z as f32 * step;

                    // Insert the voxel
                    let voxel = Voxel {
                        color: voxel_color,
                        position: Default::default(), // Will get set internally by `insert()`
                    };
                    octree.insert(wx, wy, wz, voxel);
                }
            }
        }
    }
}



/// Inserts a 16x256x16 "column" of voxels into the octree at (0,0,0) corner.
/// If you want it offset or centered differently, just adjust the for-loop ranges or offsets.
fn generate_voxel_rect(
    octree: &mut SparseVoxelOctree,
    voxel_color: Color,
) {
    // The dimensions of our rectangle: 16 x 256 x 16
    let size_x = 16;
    let size_y = 256;
    let size_z = 16;

    // We'll get the voxel spacing (size at the deepest level), same as in your sphere code.
    let step = octree.get_spacing_at_depth(octree.max_depth);

    // Triple-nested loop for each voxel in [0..16, 0..256, 0..16]
    for ix in 0..size_x {
        let x = ix as f32;
        for iy in 0..size_y {
            let y = iy as f32;
            for iz in 0..size_z {
                let z = iz as f32;

                // Convert (x,y,z) to world coordinates
                let wx = x * step;
                let wy = y * step;
                let wz = z * step;

                // Create the voxel
                let voxel = Voxel {
                    color: voxel_color,
                    position: Default::default(), // Will be set by octree internally
                };

                // Insert the voxel into the octree
                octree.insert(wx, wy, wz, voxel);
            }
        }
    }
}

fn generate_large_plane(
    octree: &mut SparseVoxelOctree,
    width: usize,
    depth: usize,
    color: Color,
) {
    // We'll get the voxel spacing (size at the deepest level).
    let step = octree.get_spacing_at_depth(octree.max_depth);

    // Double-nested loop for each voxel in [0..width, 0..depth],
    // with y=0. 
    for ix in 0..width {
        let x = ix as f32;
        for iz in 0..depth {
            let z = iz as f32;
            // y is always 0. 
            let y = 0.0;

            // Convert (x,0,z) to world coordinates
            let wx = x * step;
            let wy = y * step;
            let wz = z * step;

            // Create the voxel
            let voxel = Voxel {
                color,
                position: Vec3::ZERO, // Will be set internally by octree.insert()
            };

            // Insert the voxel into the octree
            octree.insert(wx, wy, wz, voxel);
        }
    }
}





