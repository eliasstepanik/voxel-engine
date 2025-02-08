
use bevy::app::{App, Plugin, PreUpdate, Startup, Update};
use bevy::prelude::IntoSystemConfigs;
use crate::helper::large_transform::*;

pub struct LargeTransformPlugin;
impl Plugin for LargeTransformPlugin {
    fn build(&self, app: &mut App) {
        
        app.insert_resource(WorldOffset::default());
        app.add_systems(Startup, setup);
        app.add_systems(Update, floating_origin_system.after(crate::systems::camera_system::camera_controller_system));
        app.add_systems(Update, update_render_transform_system.after(floating_origin_system));
    }


}
