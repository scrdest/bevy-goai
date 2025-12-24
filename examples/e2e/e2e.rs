use bevy_goai::*;
use goai_core::types::ActionScore;

use std::collections::HashMap;
use bevy::log::LogPlugin;
use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use serde_json;
use bevy_goai::actions::{ActionTemplate, ActionContext};
use bevy_goai::action_runtime::*;
use bevy_goai::action_state::ActionState;
use bevy_goai::actionset::ActionSet;
use bevy_goai::ai::AIController;
use bevy_goai::arg_values::ContextValue;
use bevy_goai::context_fetchers::{ContextFetcherRequest, ContextFetchResponse};
use bevy_goai::considerations::{BatchedConsiderationRequest, ConsiderationResponse, ConsiderationData};
use bevy_goai::decision_loop;
use bevy_goai::utility_concepts::{ConsiderationIdentifier, ContextFetcherIdentifier, CurveIdentifier};
use bevy_goai::smart_object::{ActionSetStore, SmartObjects};

const EXAMPLE_CONTEXT_FETCHER_NAME: &str = "ExampleCF";

#[derive(Debug, EntityEvent)]
struct ExampleActionEvent{
    /// NOTE: entity is expected to be an ActionTracker here.
    entity: Entity, 
    ctx: ActionContext,
    state: ActionState,
}

impl ExampleActionEvent {
    fn with_default_context(action_tracker: Entity) -> Self {
        Self { 
            entity: action_tracker,
            ctx: Default::default(), 
            state: ActionState::Running 
        }
    }

    fn from_context(context: ActionContext, action_tracker: Entity, state: Option<ActionState>) -> Self {
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
                bevy::log::debug!("Triggering a ExampleActionEvent...");
                commands.trigger(ExampleActionEvent::from_context(
                    tracker.0.action.context.clone(),
                    tracker_ent,
                    Some(state.0),
                ));
            },
            _ => {}
        }
    }
}

// #[derive(Event)]
// struct RunActionTrackerHandler;

// fn example_action_tracker_handler_observerized(
//     _trigger: On<RunActionTrackerHandler>,
//     query: Query<(
//         Entity, 
//         &ActionTracker, 
//         &mut ActionTrackerState, 
//         Option<&mut ActionTrackerTickTimer>
//     ), With<ActionTrackerTicks>>,
//     game_timer: Res<Time>,
//     real_timer: Res<Time<Real>>,
//     commands: Commands,
// ) {
//     example_action_tracker_handler(query, game_timer, real_timer, commands);
// }

fn example_action(
    trigger: On<ExampleActionEvent>, 
    mut commands: Commands,
) {
    let event = trigger.event();
    let tracker = event.entity;

    let tracker_cmds = commands.get_entity(tracker);

    let state = &event.state;
    let maybe_ctx = Some(&event.ctx);

    let json_state = serde_json::ser::to_string(&state);
    let state_name = json_state.unwrap();
    bevy::log::debug!("example_action: Current state is {}", state_name);

    let self_name: Option<&String> = maybe_ctx.map(|ctx| ctx.get("this").unwrap().try_into().unwrap());
    bevy::log::debug!("example_action: Self name is {:?}", self_name);

    let context_mapping = maybe_ctx.map(|ctx| ctx.get(&state_name)).flatten();

    let new: ActionState = match context_mapping {
        None => None,
        Some(cv) => {
            let clone_val = cv.clone();
            let cvstring: String = clone_val.try_into().unwrap();
            let unjsond = serde_json::de::from_str(&cvstring).unwrap();
            bevy::log::debug!("example_action: Current unjsond is {:?}", unjsond);
            unjsond
        }
    }.unwrap();

    bevy::log::debug!("example_action: New state is {:?}", new);

    match tracker_cmds {
        Err(err) => bevy::log::debug!("ActionTracker does not exist: {:?}", err),
        Ok(mut cmds) => { 
            bevy::log::debug!("example_action: Updating the ActionTracker {:?} state to new value {:?}", tracker, new);
            cmds.insert(ActionTrackerState(new)); 
        },
    }
}

fn example_context_fetcher() -> Vec<crate::actions::ActionContext> {
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
    mut consideration_registry: ResMut<decision_loop::ConsiderationKeyToSystemIdMap>,
) {
    let example_actions = [
        ActionTemplate  {
            name: "ExampleAction".to_string(),
            context_fetcher_name: ContextFetcherIdentifier(EXAMPLE_CONTEXT_FETCHER_NAME.to_string()),
            considerations: Vec::from([
                ConsiderationData::new(
                    ConsiderationIdentifier::from("ALWAYS".to_string()),
                    CurveIdentifier::from("Linear".to_string()),
                    0.,
                    1.,
                )
            ]),
            priority: 1.,
            action_key: "ExampleAction".to_string(),
        }
    ];

    let example_actionset = ActionSet {
        name: "ExampleActionSet".to_string(),
        actions: Vec::from(example_actions)
    };

    actionset_store.map_by_name.insert(example_actionset.name.to_owned(), example_actionset);
    consideration_registry.mapping.insert(
        ConsiderationIdentifier::from("ALWAYS".to_string()), 
        commands.register_system(ex_cons_one)
    );

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

    // commands.trigger(RunContextFetcherSystem);
    // commands.trigger(crate::decision_loop::TriggerAiActionScoringPhase);
    // commands.trigger(RunActionTrackerHandler);
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

fn example_context_fetcher_system(
    mut requests: MessageReader<ContextFetcherRequest>,
    mut responses: MessageWriter<ContextFetchResponse>,
) {
    // We'll return the same generic, single-option Context for all requests for now.
    // In a real scenario, this should dispatch to different user systems.

    let mut context: HashMap<String, ContextValue> = HashMap::with_capacity(3);
    // As an artifact of how we use JSON serde, we need to add escaped quotes around strings here.
    context.insert("\"Ready\"".to_string(), "\"Running\"".to_string().into());
    context.insert("\"Running\"".to_string(), "\"Failed\"".to_string().into());
    context.insert("\"Failed\"".to_string(), "\"Failed\"".to_string().into());
    context.insert("this".to_string(), EXAMPLE_CONTEXT_FETCHER_NAME.to_string().into());

    let context = Vec::from([context]);
    
    for req in requests.read() {
        bevy::log::debug!("Responding to request {:?}", req);

        responses.write(ContextFetchResponse::new(
            req.action_template.to_owned(),
            context.to_owned(),
            req.audience,
        ));
    }
}

#[derive(EntityEvent)]
struct SimpleConsiderationRequest {
    entity: Entity,
    scored_action_template: ActionTemplate,
    scored_context: ActionContext,
}

/// A trivial Consideration implementation using Events. 
/// Returns 1. (i.e. max score) for any inputs.
fn simple_consideration_impl(
    trigger: On<SimpleConsiderationRequest>,
    mut writer: MessageWriter<ConsiderationResponse>,
) {
    let req = trigger.event();
    writer.write(
        ConsiderationResponse {
            entity: req.entity,
            scored_action_template: req.scored_action_template.to_owned(),
            scored_context: req.scored_context.to_owned(),
            score: 1.,
        }
    );
}

/// A simple mock Consideration dispatch to example stuff e2e
fn example_consideration_runner(
    mut reader: MessageReader<BatchedConsiderationRequest>,
    mut commands: Commands,
) {
    for inp in reader.read() {
        for _cons in inp.considerations.iter() {
            // In a real scenario we'd match on the func_name in the Consideration
            // and dispatch to different Events based on that.
            commands.trigger(SimpleConsiderationRequest {
                entity: inp.entity,
                scored_action_template: inp.scored_action_template.to_owned(),
                scored_context: inp.scored_context.to_owned(),
            });
        }
    }
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

fn ex_cons_one(
    qry: Query<&ActionTrackerState>
) -> ActionScore {
    let mut good_cnt = 0;
    let mut bad_cnt = 0;

    for tracker in qry {
        match tracker.0 {
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
    .init_resource::<BestScoringCandidateTracker>()
    .init_resource::<decision_loop::ConsiderationKeyToSystemIdMap>()
    .add_message::<ContextFetcherRequest>()
    .add_message::<ContextFetchResponse>()
    .add_message::<BatchedConsiderationRequest>()
    .add_message::<ConsiderationResponse>()
    .register_function_with_name(EXAMPLE_CONTEXT_FETCHER_NAME, example_context_fetcher)
    .add_systems(Startup, setup_example_entity)
    .add_systems(Startup, setup_default_action_tracker_config)
    .add_observer(create_tracker_for_picked_action)
    .add_observer(actiontracker_spawn_requested)
    .add_observer(actiontracker_despawn_requested)
    .add_observer(decision_loop::ai_action_gather_phase)
    .add_observer(simple_consideration_impl)
    .add_observer(example_action)
    .add_observer(mark_despawn_occurred)
    .add_systems(FixedUpdate, (
        example_context_fetcher_system,
        decision_loop::ai_action_prescoring_phase,
        example_consideration_runner,
        decision_loop::ai_action_scoring_phase,
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
