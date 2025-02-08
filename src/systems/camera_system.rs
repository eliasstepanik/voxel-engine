use bevy::color::palettes::basic::{BLUE, GREEN};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::math::{DQuat, DVec3, Vec3};
use bevy::prelude::*;
use bevy_render::camera::{OrthographicProjection, Projection, ScalingMode};
use bevy_window::CursorGrabMode;
use crate::helper::egui_dock::MainCamera;
use crate::helper::large_transform::{DoubleTransform, WorldOffset};
use crate::InspectorVisible;
use crate::systems::voxels::structure::{Ray, SparseVoxelOctree, Voxel};

#[derive(Component)]
pub struct CameraController {
    pub yaw: f32,
    pub pitch: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            speed: 10.0,
            sensitivity: 0.1,
        }
    }
}

pub fn setup(mut commands: Commands,){



    commands.spawn((
        DoubleTransform {
            translation: DVec3::new(0.0, 0.0, 10.0),
            rotation: DQuat::IDENTITY,
            scale: DVec3::ONE,
        },
        Transform::from_xyz(0.0, 5.0, 10.0), // initial f32
        GlobalTransform::default(),
        Camera3d::default(),
        Projection::from(PerspectiveProjection{
            near: 0.0001,
          ..default()  
        }),
        MainCamera,
        CameraController::default()

        ));


}

/// Example system to control a camera using double-precision for position.
pub fn camera_controller_system(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut windows: Query<&mut Window>,
    // Here we query for BOTH DoubleTransform (f64) and Transform (f32).
    // We'll update DoubleTransform for the "true" position
    // and keep Transform in sync for rendering.a
    mut query: Query<(&mut DoubleTransform, &mut Transform, &mut CameraController)>,
    mut octree_query: Query<&mut SparseVoxelOctree>,
    mut app_exit_events: EventWriter<AppExit>,
    world_offset: Res<WorldOffset>,
) {
    let mut window = windows.single_mut();
    let (mut double_tf, mut render_tf, mut controller) = query.single_mut();

    // ====================
    // 1) Handle Mouse Look
    // ====================
    if !window.cursor_options.visible {
        for event in mouse_motion_events.read() {
            // Adjust yaw/pitch in f32
            controller.yaw -= event.delta.x * controller.sensitivity;
            controller.pitch += event.delta.y * controller.sensitivity;
            controller.pitch = controller.pitch.clamp(-89.9, 89.9);

            // Convert degrees to radians (f32)
            let yaw_radians = controller.yaw.to_radians();
            let pitch_radians = controller.pitch.to_radians();

            // Build a double-precision quaternion from those angles
            let rot_yaw = DQuat::from_axis_angle(DVec3::Y, yaw_radians as f64);
            let rot_pitch = DQuat::from_axis_angle(DVec3::X, -pitch_radians as f64);

            double_tf.rotation = rot_yaw * rot_pitch;
        }
    }

    // ====================
    // 2) Adjust Movement Speed with Mouse Wheel
    // ====================
    for event in mouse_wheel_events.read() {
        let base_factor = 1.1_f32;
        let factor = base_factor.powf(event.y);
        controller.speed *= factor;
        if controller.speed < 0.01 {
            controller.speed = 0.01;
        }
    }

    // ====================
    // 3) Handle Keyboard Movement (WASD, Space, Shift)
    // ====================
    let mut direction = DVec3::ZERO;

    // Forward/Back
    if keyboard_input.pressed(KeyCode::KeyW) {
        direction += double_tf.forward();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction -= double_tf.forward();
    }

    // Left/Right
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction -= double_tf.right();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction += double_tf.right();
    }

    // Up/Down
    if keyboard_input.pressed(KeyCode::Space) {
        direction += double_tf.up();
    }
    if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
        direction -= double_tf.up();
    }

    // Normalize direction if needed
    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
    }

    // Apply movement in double-precision
    let delta_seconds = time.delta_secs_f64();
    let distance = controller.speed as f64 * delta_seconds;
    double_tf.translation += direction * distance;

    
    
    // =========================
    // 4) Lock/Unlock Mouse (L)
    // =========================
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        // Toggle between locked and unlocked
        if window.cursor_options.grab_mode == CursorGrabMode::None {
            // Lock
            window.cursor_options.visible = false;
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
        } else {
            // Unlock
            window.cursor_options.visible = true;
            window.cursor_options.grab_mode = CursorGrabMode::None;
        }
    }



    // =======================
    // 5) Octree Keys
    // =======================
    if keyboard_input.just_pressed(KeyCode::F2){
        for mut octree in octree_query.iter_mut() {
            octree.show_wireframe = !octree.show_wireframe;
        }
    }
    if keyboard_input.just_pressed(KeyCode::F3){
        for mut octree in octree_query.iter_mut() {
            octree.show_world_grid = !octree.show_world_grid;
        }
    }
    if keyboard_input.just_pressed(KeyCode::F4){
        for mut octree in octree_query.iter_mut() {
            octree.show_chunks = !octree.show_chunks;
        }
    }
    if keyboard_input.just_pressed(KeyCode::KeyQ) && window.cursor_options.visible == false{
        for mut octree in octree_query.iter_mut() {
            octree.insert(double_tf.translation.x as f64, double_tf.translation.y as f64, double_tf.translation.z as f64, Voxel::new(Color::srgb(1.0, 0.0, 0.0)));
        }
    }

    // =======================
    // 6) Building
    // =======================

    if (mouse_button_input.just_pressed(MouseButton::Left) || mouse_button_input.just_pressed(MouseButton::Right)) && !window.cursor_options.visible {

        // Get the mouse position in normalized device coordinates (-1 to 1)
        if let Some(_) = window.cursor_position() {
            // Set the ray direction to the camera's forward vector
            let ray_origin = world_offset.0 + double_tf.translation;
            let ray_direction = double_tf.forward().normalize();

            let ray = Ray {
                origin: ray_origin.as_vec3(),
                direction: ray_direction.as_vec3(),
            };



            for mut octree in octree_query.iter_mut() {
                if let Some((hit_x, hit_y, hit_z, depth,normal)) = octree.raycast(&ray) {
                    

                    
                    /*//TODO: Currently broken needs fixing to work with double precision 
                    println!("raycast: {:?}", ray);
                    // Visualize the ray
                    lines.lines.push(EphemeralLine {
                        start: ray_origin.as_vec3(),
                        end: DVec3::new(hit_x, hit_y, hit_z).as_vec3(),
                        color: Color::from(GREEN),
                        time_left: 5.0, // draw for 2 seconds
                    });*/

                    /*gizmos.ray(
                        ray.origin,
                        ray.direction,
                        BLUE,
                    );*/
                    
                    let chunk = octree.compute_chunk_coords(hit_x, hit_y, hit_z);

                    info!("Chunk Hit: {},{},{}", chunk.0, chunk.1, chunk.2);

                    if let Some(chunk_node) = octree.get_chunk_node(hit_x,hit_y,hit_z) {
                        let has_volume = octree.has_volume(chunk_node);

                        info!("Chunk Has Volume: {}", has_volume);
                    }
                    
                    




                    


                    if mouse_button_input.just_pressed(MouseButton::Right) {

                        let voxel_size = octree.get_spacing_at_depth(depth);
                        let hit_position = Vec3::new(hit_x as f32, hit_y as f32, hit_z as f32);
                        let epsilon = voxel_size * 0.1; // Adjust this value as needed (e.g., 0.1 times the voxel size)

                        // Offset position by epsilon in the direction of the normal
                        let offset_position = hit_position - (normal * Vec3::new(epsilon as f32, epsilon as f32, epsilon as f32));

                        // Align the offset position to the center of the nearest voxel
                        let (new_voxel_x, new_voxel_y, new_voxel_z) = octree.normalize_to_voxel_at_depth(
                            offset_position.x as f64,
                            offset_position.y as f64,
                            offset_position.z as f64,
                            depth,
                        );

                        // Remove the voxel
                        octree.remove(new_voxel_x, new_voxel_y, new_voxel_z);
                    }
                    else if mouse_button_input.just_pressed(MouseButton::Left) {

                        let voxel_size = octree.get_spacing_at_depth(depth);
                        let hit_position = Vec3::new(hit_x as f32, hit_y as f32, hit_z as f32);
                        let epsilon = voxel_size * 0.1; // Adjust this value as needed (e.g., 0.1 times the voxel size)

                        // Offset position by epsilon in the direction of the normal
                        let offset_position = hit_position + (normal * Vec3::new(epsilon as f32, epsilon as f32, epsilon as f32));

                        // Align the offset position to the center of the nearest voxel
                        let (new_voxel_x, new_voxel_y, new_voxel_z) = octree.normalize_to_voxel_at_depth(
                            offset_position.x as f64,
                            offset_position.y as f64,
                            offset_position.z as f64,
                            depth,
                        );

                        // Insert the new voxel
                        octree.insert(
                            new_voxel_x,
                            new_voxel_y,
                            new_voxel_z,
                            Voxel::new(Color::srgb(1.0, 0.0, 0.0)),
                        );
                    }
                }
            }
        }
    }


    // =======================
    // 7) Exit on Escape
    // =======================
    if keyboard_input.pressed(KeyCode::Escape) {
        app_exit_events.send(Default::default());
    }

    // =============================================
    // 8) Convert DoubleTransform -> Bevy Transform
    // =============================================
    // The final step is to update the f32 `Transform` that Bevy uses for rendering.
    // This ensures the camera is visually placed at the correct position.
    render_tf.translation = double_tf.translation.as_vec3();
    render_tf.rotation = double_tf.rotation.as_quat();
    render_tf.scale = double_tf.scale.as_vec3();
        
    
}