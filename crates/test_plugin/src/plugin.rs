use bevy::log::LogPlugin;
use bevy::{app::ScheduleRunnerPlugin, prelude::*};

use crate::helpers::*;


pub struct CortexTestPlugin; 

impl Plugin for CortexTestPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(200))),
            LogPlugin { 
                level: bevy::log::Level::DEBUG, 
                custom_layer: |_| None, 
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
                fmt_layer: |_| None,
            }
        ))
        .init_resource::<DespawnedAnyActionTrackers>()
        .add_observer(mark_despawn_occurred)
        .add_systems(
            Last, 
            (
                exit_on_finish_all_tasks,
            ).chain()
        )
        ;
    }
}
