use bevy::prelude::*;
use cortex_core::action_runtime;
use cortex_core::considerations;
use cortex_core::context_fetchers;
use cortex_core::decision_loop::decision_engine;
use cortex_core::smart_object;

pub struct CortexPlugin; 

impl Plugin for CortexPlugin {
    fn build(&self, app: &mut App) {
        app
        .init_resource::<action_runtime::UserDefaultActionTrackerSpawnConfig>()
        .init_resource::<smart_object::ActionSetStore>()
        // Technically unnecessary, but will give users saner error messages if we pre-initialize:
        .init_resource::<context_fetchers::ContextFetcherKeyToSystemMap>()
        // Technically unnecessary, but will give users saner error messages if we pre-initialize:
        .init_resource::<considerations::ConsiderationKeyToSystemMap>()
        .add_observer(action_runtime::create_tracker_for_picked_action)
        .add_observer(action_runtime::actiontracker_spawn_requested)
        .add_observer(action_runtime::actiontracker_despawn_requested)
        .add_observer(decision_engine)
        .add_systems(
            FixedPostUpdate, 
            (
                action_runtime::actiontracker_done_cleanup_system,
            ).chain()
        )
        ;
    }
}
