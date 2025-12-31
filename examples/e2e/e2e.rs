use bevy_cortex::*;
use goai_core::types::{ActionContextRef, ActionScore};

use std::collections::HashMap;
use bevy::log::LogPlugin;
use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use serde_json;
use bevy_cortex::actions::{ActionTemplate};
use bevy_cortex::action_runtime::*;
use bevy_cortex::action_state::ActionState;
use bevy_cortex::actionset::ActionSet;
use bevy_cortex::ai::AIController;
use bevy_cortex::arg_values::ContextValue;
use bevy_cortex::context_fetchers::{ContextFetcherRequest, ContextFetchResponse, AcceptsContextFetcherRegistrations};
use bevy_cortex::considerations::{BatchedConsiderationRequest, ConsiderationData, AcceptsConsiderationRegistrations};
use bevy_cortex::decision_loop;
use bevy_cortex::utility_concepts::{ConsiderationIdentifier, ContextFetcherIdentifier, CurveIdentifier};
use bevy_cortex::smart_object::{ActionSetStore, SmartObjects};

const EXAMPLE_CONTEXT_FETCHER_NAME: &str = "ExampleCF";

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
            "ExampleAction" => {
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

    let state = &event.state;
    let maybe_ctx = Some(&event.ctx);

    let json_state = serde_json::ser::to_string(&state);
    let state_name = json_state.unwrap();
    bevy::log::info!("example_action for AI {:?}: Current state is {}", ai_owner, state_name);

    let self_name: Option<&String> = maybe_ctx.map(|ctx| ctx.get("this").unwrap().try_into().unwrap());
    bevy::log::debug!("example_action for AI {:?}: Self name is {:?}", ai_owner, self_name);

    let context_mapping = maybe_ctx.map(|ctx| ctx.get(&state_name)).flatten();

    let new: ActionState = match context_mapping {
        None => None,
        Some(cv) => {
            let clone_val = cv.clone();
            let cvstring: String = clone_val.try_into().unwrap();
            let unjsond = serde_json::de::from_str(&cvstring).unwrap();
            bevy::log::debug!("example_action for AI {:?}: Current unjsond is {:?}", ai_owner, unjsond);
            unjsond
        }
    }.unwrap();

    bevy::log::info!("example_action for AI {:?}: New state is \"{:?}\"", ai_owner, new);

    match maybe_tracker_state {
        Err(err) => bevy::log::debug!("ActionTracker does not exist: {:?}", err),
        Ok((upd_tracker, mut state)) => { 
            bevy::log::debug!("example_action for AI {:?}: Updating the ActionTracker {:?} state to new value {:?}", ai_owner, upd_tracker, new);
            state.set_state(new);
        },
    }
}

fn example_context_fetcher(
    _inp: crate::types::ContextFetcherInputs
) -> crate::types::ContextFetcherOutputs {
    let mut context: HashMap<String, ContextValue> = HashMap::with_capacity(3);
    // As an artifact of how we use JSON serde, we need to add escaped quotes around strings here.
    context.insert("\"Ready\"".to_string(), "\"Running\"".to_string().into());
    context.insert("\"Running\"".to_string(), "\"Failed\"".to_string().into());
    context.insert("\"Failed\"".to_string(), "\"Failed\"".to_string().into());
    context.insert("this".to_string(), EXAMPLE_CONTEXT_FETCHER_NAME.to_string().into());
    Vec::from([context])
}

fn setup_example_entity(
    mut commands: Commands,
    mut actionset_store: ResMut<ActionSetStore>,
) {
    let example_actions = [
        ActionTemplate  {
            name: "ExampleAction".to_string(),
            context_fetcher_name: ContextFetcherIdentifier(EXAMPLE_CONTEXT_FETCHER_NAME.to_string()),
            considerations: Vec::from([
                ConsiderationData::new(
                    ConsiderationIdentifier::from("One".to_string()),
                    CurveIdentifier::from("Linear".to_string()),
                    0.,
                    1.5,
                ),
                ConsiderationData::new(
                    ConsiderationIdentifier::from("Two".to_string()),
                    CurveIdentifier::from("Square".to_string()),
                    0.,
                    1.,
                )
            ]),
            priority: 1.,
            action_key: "ExampleAction".to_string(),
            lod_min: None, 
            lod_max: None,
        }
    ];

    let example_actionset = ActionSet {
        name: "ExampleActionSet".to_string(),
        actions: Vec::from(example_actions)
    };

    actionset_store.map_by_name.insert(example_actionset.name.to_owned(), example_actionset);

    let new_controller = AIController::default();
    let new_sos = SmartObjects {
        actionset_refs: Vec::from(["ExampleActionSet".to_string()])
    };

    let spawned = commands.spawn((
        new_controller,
    ));

    let ai_id = spawned.id();

    commands.trigger(crate::events::AiDecisionRequested { 
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
    .init_resource::<UserDefaultActionTrackerSpawnConfig>()
    .init_resource::<ActionSetStore>()
    .init_resource::<DespawnedAnyActionTrackers>()
    .register_consideration(example_consideration_one, "One".into())
    .register_consideration(example_consideration_two, "Two".into())
    .register_context_fetcher(example_context_fetcher, EXAMPLE_CONTEXT_FETCHER_NAME.to_string().into())
    .add_message::<ContextFetcherRequest>()
    .add_message::<ContextFetchResponse>()
    .add_message::<BatchedConsiderationRequest>()
    .add_systems(Startup, (
        setup_example_entity, 
        setup_default_action_tracker_config,
    ))
    .add_observer(create_tracker_for_picked_action)
    .add_observer(actiontracker_spawn_requested)
    .add_observer(actiontracker_despawn_requested)
    .add_observer(decision_loop::decision_engine)
    .add_observer(example_action)
    .add_observer(mark_despawn_occurred)
    .add_systems(FixedUpdate, (
        example_action_tracker_handler,
    ).chain())
    .add_systems(
        FixedPostUpdate, 
        (
            actiontracker_done_cleanup_system,
            exit_on_finish_all_tasks,
        ).chain()
    )
    ;

    app.run();
}
