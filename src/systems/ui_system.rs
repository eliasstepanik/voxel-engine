use bevy::asset::AssetServer;
use bevy::prelude::*;
use crate::helper::large_transform::{DoubleTransform, WorldOffset};
use crate::systems::camera_system::CameraController;
use crate::systems::voxels::structure::{SparseVoxelOctree};

#[derive(Component)]
pub struct SpeedDisplay;

/// Spawns a UI Text entity to display speed/positions.
pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Use the new UI API, or the old UI Node-based system. 
    // This example uses an older approach to Node/Style, but can be adapted to `TextBundle`.
    // If you're on Bevy 0.11+, you can also do `TextBundle::from_section(...)`.
    commands.spawn((
        // The text to display:
        Text::new("Speed: 0.0"),
        // The font, loaded from an asset file
        TextFont {
            font: asset_server.load("fonts/minecraft_font.ttf"),
            font_size: 25.0,
            ..default()
        },
        // The text layout style
        TextLayout::new_with_justify(JustifyText::Left),
        // Style for positioning the UI node
        Node {
            position_type: PositionType::Relative,
            bottom: Val::Px(9.0),
            right: Val::Px(9.0),
            ..default()
        },
        // Our marker so we can query this entity
        SpeedDisplay,
    ));
}

/// System that updates the UI text each frame with 
///  - speed
///  - camera f32 position
///  - camera global f64 position
///  - current chunk coordinate
pub fn update(
    // Query the camera controller so we can see its speed
    query_camera_controller: Query<&CameraController>,
    // We also query for the camera's f32 `Transform` and the double `DoubleTransform`
    camera_query: Query<(&Transform, &DoubleTransform, &Camera)>,
    // The global offset resource, if you have one
    world_offset: Res<WorldOffset>,
    // The chunk-size logic from the octree, so we can compute chunk coords
    octree_query: Query<&SparseVoxelOctree>, // or get_single if there's only one octree

    // The UI text entity
    mut query_text: Query<&mut Text, With<SpeedDisplay>>,
) {
    let camera_controller = query_camera_controller.single();
    let (transform, double_tf, _camera) = camera_query.single();
    let mut text = query_text.single_mut();

    // The global double position: offset + camera's double translation
    let global_pos = world_offset.0 + double_tf.translation;

    // We'll attempt to get the octree so we can compute chunk coords
    // If there's no octree, we just show "N/A".
    /*let (chunk_cx, chunk_cy, chunk_cz) = if let Ok(octree) = octree_query.get_single() {
        // 1) get voxel step
        let step = octree.get_spacing_at_depth(octree.max_depth);
        // 2) chunk world size
        let chunk_world_size = CHUNK_SIZE as f64 * step;
        // 3) compute chunk coords using global_pos
        let cx = ((global_pos.x) / chunk_world_size).floor() as i32;
        let cy = ((global_pos.y) / chunk_world_size).floor() as i32;
        let cz = ((global_pos.z) / chunk_world_size).floor() as i32;
        (cx, cy, cz)
    } else {
        (0, 0, 0) // or default
    };*/

    // Format the string to show speed, positions, and chunk coords
    text.0 = format!(
        "\n  Speed: {:.3}\n  Position(f32): ({:.2},{:.2},{:.2})\n  Position(f64): ({:.2},{:.2},{:.2})",
        camera_controller.speed,
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
        global_pos.x,
        global_pos.y,
        global_pos.z,
    );
}