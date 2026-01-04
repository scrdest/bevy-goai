use bevy::prelude::*;
use cortex_core::action_runtime;
use cortex_core::considerations;
use cortex_core::context_fetchers;
use cortex_core::decision_loop;
use cortex_core::smart_object;

#[cfg(feature = "include_actionset_loader")]
use cortex_actionset_loader::ActionSetAssetPlugin;

pub struct CortexPlugin; 

impl Plugin for CortexPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "include_actionset_loader")]
        app
        .add_plugins((
            ActionSetAssetPlugin, 
        ));

        app
        .add_plugins((
            context_fetchers::ContextFetcherPlugin, 
            considerations::ConsiderationPlugin,
        ))
        .init_resource::<action_runtime::UserDefaultActionTrackerSpawnConfig>()
        .init_resource::<smart_object::ActionSetStore>()
        .add_observer(action_runtime::create_tracker_for_picked_action)
        .add_observer(action_runtime::actiontracker_triggered_spawner)
        .add_observer(action_runtime::actiontracker_triggered_despawner)
        .add_observer(decision_loop::prepare_ai)
        .add_observer(decision_loop::decision_engine)
        .add_systems(
            FixedPostUpdate, 
            (
                action_runtime::actiontracker_done_cleanup_system,
            ).chain()
        )
        ;
    }
}
