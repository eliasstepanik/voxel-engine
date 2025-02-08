use std::fs::create_dir;
use bevy::app::{App, Plugin, PreUpdate, Startup};
use bevy::color::palettes::css::{GRAY, RED};
use bevy::prelude::{default, Color, Commands, GlobalTransform, IntoSystemConfigs, Query, Res, Update};
use bevy_render::prelude::ClearColor;
use crate::app::InspectorVisible;
use crate::systems::environment_system::*;
use crate::systems::voxels::structure::{ChunkEntities, SparseVoxelOctree, Voxel};

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        /*app.insert_resource(ClearColor(Color::from(GRAY)));*/
        app.init_resource::<ChunkEntities>();
        app.add_systems(Startup, (setup).chain());
        app.add_systems(Update, (crate::systems::voxels::rendering::render,crate::systems::voxels::debug::visualize_octree.run_if(should_visualize_octree), crate::systems::voxels::debug::draw_grid.run_if(should_draw_grid), crate::systems::voxels::debug::debug_draw_chunks_system.run_if(should_visualize_chunks)).chain());

        app.register_type::<SparseVoxelOctree>();
        app.register_type::<ChunkEntities>();

    }



}

fn should_visualize_octree(octree_query: Query<&SparseVoxelOctree>,) -> bool {
    octree_query.single().show_wireframe
}

fn should_draw_grid(octree_query: Query<&SparseVoxelOctree>,) -> bool {
    octree_query.single().show_world_grid
}

fn should_visualize_chunks(octree_query: Query<&SparseVoxelOctree>,) -> bool {
    octree_query.single().show_chunks
}