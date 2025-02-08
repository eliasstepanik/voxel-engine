use bevy::math::{DQuat, DVec3};
use bevy::prelude::{Commands, Component, GlobalTransform, Query, Reflect, Res, ResMut, Resource, Transform, With, Without};
use bevy_render::prelude::Camera;


#[derive(Resource, Reflect,Default)]
pub struct WorldOffset(pub DVec3);

#[derive(Component, Default,Reflect)]
pub struct DoubleTransform {
    pub translation: DVec3,
    pub rotation: DQuat,
    pub scale: DVec3,
}

impl DoubleTransform {
    pub fn new(translation: DVec3, rotation: DQuat, scale: DVec3) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Returns a unit vector pointing "forward" (negative-Z) based on the rotation
    pub fn forward(&self) -> DVec3 {
        self.rotation * DVec3::new(0.0, 0.0, -1.0)
    }

    /// Returns a unit vector pointing "right" (positive-X)
    pub fn right(&self) -> DVec3 {
        self.rotation * DVec3::new(1.0, 0.0, 0.0)
    }

    /// Returns a unit vector pointing "up" (positive-Y)
    pub fn up(&self) -> DVec3 {
        self.rotation * DVec3::new(0.0, 1.0, 0.0)
    }
    pub fn down(&self) -> DVec3 {
        self.rotation * DVec3::new(0.0, -1.0, 0.0)
    }

}

pub(crate) fn get_true_world_position(
    offset: &WorldOffset,
    transform: &DoubleTransform,
) -> DVec3 {
    transform.translation + offset.0
}

pub fn setup(mut commands: Commands) {
    commands
        .spawn((
            DoubleTransform {
                translation: DVec3::new(100_000.0, 0.0, 0.0),
                rotation: DQuat::IDENTITY,
                scale: DVec3::ONE,
            },
            // The standard Bevy Transform (will be updated each frame)
            Transform::default(),
            GlobalTransform::default(),
            // Add your mesh/visibility components, etc.
        ));

}


pub fn update_render_transform_system(
    camera_query: Query<&DoubleTransform, With<Camera>>,
    mut query: Query<(&DoubleTransform, &mut Transform), Without<Camera>>,
) {
    let camera_double_tf = camera_query.single();
    // The camera offset in double-precision
    let camera_pos = camera_double_tf.translation;

    for (double_tf, mut transform) in query.iter_mut() {
        // relative position (double precision)
        let relative_pos = double_tf.translation - camera_pos;
        transform.translation = relative_pos.as_vec3(); // convert f64 -> f32
        transform.rotation = double_tf.rotation.as_quat(); // f64 -> f32
        transform.scale = double_tf.scale.as_vec3();       // f64 -> f32
    }
}

pub fn floating_origin_system(
    mut query: Query<&mut DoubleTransform, Without<Camera>>,
    mut camera_query: Query<&mut DoubleTransform, With<Camera>>,
    mut offset: ResMut<WorldOffset>,
) {
    let mut camera_tf = camera_query.single_mut();
    let camera_pos = camera_tf.translation;

    // If the camera moves any distance, recenter it
    if camera_pos.length() > 0.001 {
        offset.0 += camera_pos;
        // Shift everything so camera ends up back at zero
        for mut dtf in query.iter_mut() {
            dtf.translation -= camera_pos;
        }
        camera_tf.translation = DVec3::ZERO;
    }
}