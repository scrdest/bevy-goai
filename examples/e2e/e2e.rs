use std::collections::HashMap;

use bevy::log::LogPlugin;
use bevy::{app::ScheduleRunnerPlugin, prelude::*};

use bevy_cortex::actions::{ActionTemplate};
use bevy_cortex::action_runtime::*;
use bevy_cortex::action_state::ActionState;
use bevy_cortex::actionset::ActionSet;
use bevy_cortex::ai::AIController;
use bevy_cortex::context_fetchers::{AcceptsContextFetcherRegistrations};
use bevy_cortex::considerations::{ConsiderationData, AcceptsConsiderationRegistrations};
use bevy_cortex::events;
use bevy_cortex::types::{self, ActionContextRef, ActionScore, ThreadSafeRef};
use bevy_cortex::smart_object::{ActionSetStore, SmartObjects};
use bevy_cortex::prelude::CortexPlugin;

const EXAMPLE_CONTEXT_FETCHER_NAME: &str = "e2e::ExampleCF";

#[derive(Debug, EntityEvent)]
struct ExampleActionEvent{
    /// NOTE: entity is expected to be an ActionTracker here.
    entity: Entity, 
    ctx: ActionContextRef,
    state: ActionState,
}

impl ExampleActionEvent {
    fn from_context_ref(context: ActionContextRef, action_tracker: Entity, state: Option<ActionState>) -> Self {
        Self {
            entity: action_tracker,
            ctx: context,
            state: state.unwrap_or(ActionState::Ready),
        }
    }
}

/// Mockup of user application code - dispatches actual execution Events 
/// based on the key in the library Event to user-implemented Action Systems.
/// This specific implementation uses the tick-based Action processing API.
fn example_action_tracker_handler(
    mut query: Query<(
        Entity, 
        &ActionTracker, 
        &mut ActionTrackerState, 
        Option<&mut ActionTrackerTickTimer>
    ), With<ActionTrackerTicks>>,
    game_timer: Res<Time>,
    real_timer: Res<Time<Real>>,
    mut commands: Commands,
) {
    for (tracker_ent, tracker, state, tick_timer) in query.iter_mut() {
        if !state.0.should_process() {
            bevy::log::debug!("Skipping processing for Action(Tracker) {:?} - {:?}", tracker.0.action.name, state.0);
            continue;
        }

        if let Some(mut tick_timer_included) = tick_timer {
            let current_time_game = game_timer.elapsed();
            let current_time_real = real_timer.elapsed();

            let new_value = TimeInstantActionTracker::VirtualAndReal((
                current_time_game, current_time_real
            ));

            tick_timer_included.last_tick_time = Some(new_value);
        }
        
        let action_key = &tracker.0.action.action_key;

        match action_key.as_str() {
            "e2e::ExampleAction" => {
                bevy::log::info!("Triggering a ExampleActionEvent...");
                commands.trigger(ExampleActionEvent::from_context_ref(
                    tracker.0.action.context.clone(),
                    tracker_ent,
                    Some(state.0),
                ));
            },
            _ => {}
        }
    }
}

fn example_action(
    trigger: On<ExampleActionEvent>, 
    associated_ai_qry: Query<(&ActionTracker, &ActionTrackerOwningAI)>,
    context_data_qry: Query<&ExampleStateMapContextComponent>,
    mut tracker_state_qry: Query<(&ActionTracker, &mut ActionTrackerState)>,
) {
    let event = trigger.event();

    let tracker = event.entity;

    let maybe_tracker_state = tracker_state_qry.get_mut(tracker);
    let maybe_ai_owner = associated_ai_qry
        .get(tracker)
        .ok()
        .map(|bundle| &bundle.1.owner_ai)
    ;

    let ai_owner = maybe_ai_owner
        .map(|own| format!("{:}", own))
        .unwrap_or("<none>".to_string())
    ;

    let context_data = match context_data_qry.get(event.ctx) {
        Ok(data) => data,
        Err(err) => {
            bevy::log::error!(
                "example_action for AI {:?} - Received an invalid Context {:?} ({:?})!", 
                ai_owner, event.ctx, err,
            );
            // panic!("Invalid context!");
            return;
        }
    };

    let state_mapping = &context_data.statemap;
    let state = &event.state;

    bevy::log::info!("example_action for AI {:?} - Current state is {:?}", ai_owner, state);

    let new = match state_mapping.get(&state) {
        None => {
            bevy::log::error!(
                "example_action for AI {:?} - could not find a mapped target state for current state {:?}, aborting!", 
                ai_owner, state
            );
            return;
        },
        Some(new_state) => new_state
    };

    bevy::log::info!("example_action for AI {:?}: New state is {:?}", ai_owner, new);

    match maybe_tracker_state {
        Err(err) => bevy::log::debug!("ActionTracker does not exist: {:?}", err),
        Ok((upd_tracker, mut state)) => { 
            bevy::log::debug!("example_action for AI {:?}: Updating the ActionTracker {:?} state to new value {:?}", ai_owner, upd_tracker, new);
            state.set_state(*new);
        },
    }
}

/// A Component storing a simple FSM-like state transition mapping. 
/// 
/// Only used for demonstration purposes; we'll add an example ContextFetcher 
/// that returns Entities with this Component and use the Component in a sample 
/// Action which will use the mapping to figure out its next State by lookup.
#[derive(Component, Default, Debug, Clone)]
struct ExampleStateMapContextComponent {
    statemap: HashMap<ActionState, ActionState>
}

#[derive(Component, Default, Debug, Clone)]
pub struct DumbMarker;

/// A ContextFetcher that returns Entities with an ExampleStateMapContextComponent
fn example_context_fetcher(
    inp: crate::types::ContextFetcherInputs,
    context_data_qry: Query<(Entity, &DumbMarker)>,
) -> crate::types::ContextFetcherOutputs {
    bevy::log::debug!("example_context_fetcher triggered for AI {:?}", inp.0.0);
    context_data_qry.iter().filter_map(|(id, map)| {
        // bevy::log::debug!("example_context_fetcher for AI {:?} processing Entity {:?} w/ {:?}, {:?}", inp.0.0, id, world.);
        bevy::log::debug!("example_context_fetcher for AI {:?} processing Entity {:?} w/ {:?}", inp.0.0, id, map);
        Some(id)
    }).collect()
}

fn setup_example_context(
    mut commands: Commands,
) {
    let mut statemap = HashMap::with_capacity(3);
    statemap.insert(ActionState::Ready, ActionState::Running);
    statemap.insert(ActionState::Running, ActionState::Failed);
    statemap.insert(ActionState::Failed, ActionState::Failed);

    let component = ExampleStateMapContextComponent {statemap};
    let backup = component.clone();

    let newent = commands.spawn((
        component,
        DumbMarker::default(),
    ));
    let ctx_id = newent.id();

    commands.spawn((
        backup.clone(),
        DumbMarker::default(),
    ));

    bevy::log::debug!("setup_example_context - Spawned a Context entity with ID {:?} w/ {:?}", ctx_id, backup)
}

fn setup_example_entity(
    mut commands: Commands,
    mut actionset_store: ResMut<ActionSetStore>,
) {
    let example_actions = [
        ActionTemplate::new(
            "ExampleAction",
            EXAMPLE_CONTEXT_FETCHER_NAME.to_string(), 
            Vec::from([
                ConsiderationData::new(
                    "e2e::One",
                    "Linear",
                    0.,
                    1.5,
                ),
                ConsiderationData::new(
                    "e2e::Two",
                    "Square",
                    0.,
                    1.,
                )
            ]),
            1.,
            "e2e::ExampleAction",
            None, 
            None,
        )
    ];

    let example_actionset = ActionSet {
        name: "ExampleActionSet".to_string(),
        actions: Vec::from(example_actions)
    };

    actionset_store.map_by_name.insert(example_actionset.name.to_owned(), example_actionset);

    let new_controller = AIController::default();
    let new_sos = SmartObjects {
        actionset_refs: ThreadSafeRef::new(Vec::from(["ExampleActionSet".to_string()]))
    };

    let spawned = commands.spawn((
        new_controller,
    ));

    let ai_id = spawned.id();

    commands.trigger(events::AiDecisionRequested { 
        entity: ai_id,  
        smart_objects: Some(new_sos),
    });
}

fn setup_default_action_tracker_config(
    mut config_res: ResMut<UserDefaultActionTrackerSpawnConfig>
) {
    let new_config = 
        ActionTrackerSpawnConfigBuilder::new()
        .set_use_ticker(true)
        .set_use_timers(false)
    ;
    config_res.config = Some(new_config.build());
}

/// Helper for triggering AppExit. 
/// If True, we have actually despawned at least one tracker.
/// This is needed so we don't stop BEFORE any trackers are created.
#[derive(Resource)]
struct DespawnedAnyActionTrackers(bool);

impl Default for DespawnedAnyActionTrackers {
    fn default() -> Self {
        Self(false)
    }
}

/// Helper for triggering AppExit. 
/// Updates the DespawnedAnyActionTrackers on a despawn.
fn mark_despawn_occurred(
    _trigger: On<ActionTrackerDespawnRequested>,
    mut marker: ResMut<DespawnedAnyActionTrackers>
) {
    marker.0 = true;
}

/// Calls AppExit if there are no running tasks left and at least one has been despawned.
/// This is a helper to make it easier to test in a loop without manually killing the app.
fn exit_on_finish_all_tasks(
    query: Query<&ActionTracker>,
    despawned: Res<DespawnedAnyActionTrackers>,
    mut exit_writer: MessageWriter<AppExit>,
) {
    if !despawned.0 {
        return;
    }

    if query.is_empty() {
        exit_writer.write(AppExit::Success);
    }
}

fn example_consideration_one(
    _inputs: types::ConsiderationInputs,
    qry: Query<&ActionTrackerState>
) -> ActionScore {
    let mut good_cnt = 0;
    let mut bad_cnt = 0;

    for tracker in qry {
        match tracker.get_state() {
            ActionState::Failed => { bad_cnt += 1 },
            _ => { good_cnt += 1 },
        }
    }

    let total_cnt = good_cnt + bad_cnt;

    if total_cnt > 0 {
        (good_cnt as ActionScore) / (total_cnt as ActionScore)
    } else {
        1.
    }
}

/// Trivial Consideration, returns a flat value.
fn example_consideration_two(
    _inputs: types::ConsiderationInputs,
) -> ActionScore {
    0.9
}

fn main() {
    let mut app = App::new();

    app
    .add_plugins((
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(200))),
        // MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
        LogPlugin { 
            level: bevy::log::Level::DEBUG, 
            custom_layer: |_| None, 
            filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
            fmt_layer: |_| None,
        }
    ))
    .add_plugins(CortexPlugin)
    .init_resource::<DespawnedAnyActionTrackers>()
    .register_consideration(example_consideration_one, "e2e::One")
    .register_consideration(example_consideration_two, "e2e::Two")
    .register_context_fetcher(example_context_fetcher, EXAMPLE_CONTEXT_FETCHER_NAME)
    .add_systems(Startup, (
        setup_example_context, 
        setup_example_entity, 
        setup_default_action_tracker_config,
    ))
    .add_observer(example_action)
    .add_observer(mark_despawn_occurred)
    .add_systems(FixedUpdate, (
        example_action_tracker_handler,
    ).chain())
    .add_systems(
        FixedPostUpdate, 
        (
            exit_on_finish_all_tasks,
        ).chain()
    )
    ;

    app.run();
}
