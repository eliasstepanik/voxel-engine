
use bevy::app::{App, Plugin, Startup};
use bevy::color::palettes::basic::{GREEN, YELLOW};
use bevy::color::palettes::css::RED;
use bevy::prelude::*;
use crate::systems::environment_system::*;
use crate::systems::voxels::structure::{OctreeNode, SparseVoxelOctree};

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {

        app.add_systems(Startup, (setup).chain());
        app.add_systems(Update, (crate::systems::voxels::rendering::render,crate::systems::voxels::debug::visualize_octree.run_if(should_visualize_octree), crate::systems::voxels::debug::draw_grid.run_if(should_draw_grid)).chain());

        app.register_type::<SparseVoxelOctree>();

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

