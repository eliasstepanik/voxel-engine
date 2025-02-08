use bevy::a11y::AccessibilitySystem::Update;
use bevy::app::{App, Plugin, PreUpdate, Startup};

pub struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (crate::systems::camera_system::setup));
        app.add_systems(PreUpdate, (crate::systems::camera_system::camera_controller_system));
    }


}
