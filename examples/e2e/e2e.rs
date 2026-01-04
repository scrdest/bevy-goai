use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use cortex::actions::{ActionTemplate};
use cortex::action_runtime::*;
use cortex::action_state::ActionState;
use cortex::actionset::ActionSet;
use cortex::ai::AIController;
use cortex::context_fetchers::{AcceptsContextFetcherRegistrations};
use cortex::considerations::{ConsiderationData, AcceptsConsiderationRegistrations};
use cortex::curves::{AcceptsCurveRegistrations, LinearCurve, UtilityCurveExt};
use cortex::events;
use cortex::types::{self, ActionContextRef, ActionScore, ConsiderationOutputs, ThreadSafeRef};
use cortex::smart_object::{ActionSetStore, SmartObjects};

use cortex_bevy_plugin::CortexPlugin;
use cortex_test_plugin::CortexTestPlugin;

const EXAMPLE_CONTEXT_FETCHER_NAME: &str = "e2e::ExampleCF";

#[derive(Debug, EntityEvent)]
struct ExampleActionEvent{
    entity: types::AiEntity, 
    _pawn: types::PawnEntityRef, 
    ctx: types::ActionContextRef,
    state: ActionState,
}

impl ExampleActionEvent {
    fn from_context_ref(
        context: ActionContextRef, 
        ai: types::AiEntity, 
        pawn: types::PawnEntityRef,
        state: Option<ActionState>
    ) -> Self {
        Self {
            entity: ai,
            _pawn: pawn,
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
        &ActionTrackerOwningAI, 
        &mut ActionTrackerState, 
        Option<&mut ActionTrackerTickTimer>
    ), With<ActionTrackerTicks>>,
    game_timer: Res<Time>,
    real_timer: Res<Time<Real>>,
    mut commands: Commands,
) {
    bevy::log::debug!("example_action_tracker_handler: Running...");
    for (tracker_ent, tracker, ai_tracker, state, tick_timer) in query.iter_mut() {
        if !state.0.should_process() {
            bevy::log::debug!(
                "example_action_tracker_handler - AI {:?}: Skipping processing for Action(Tracker) {:?} - {:?}", 
                ai_tracker.owner_ai, tracker.0.action.name, state.0
            );
            continue;
        }

        bevy::log::debug!(
            "example_action_tracker_handler: processing Action(Tracker) {:?} - {:?}", 
            tracker.0.action.name, state.0
        );

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
                    None,
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
    bevy::log::debug!("example_action: Running...");
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
            panic!("Invalid context!");
            // return;
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


/// A simple 2d Position component for demo purposes only.
#[derive(Component, Clone, Copy, Debug, Default)]
struct Position2d(Vec2);

impl Position2d {
    fn euclid_distance(&self, other: &Self) -> f32 {
        self.0.distance(other.0)
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
    context_data_qry: Query<Entity, With<ExampleStateMapContextComponent>>,
) -> crate::types::ContextFetcherOutputs {
    bevy::log::debug!("example_context_fetcher triggered for AI {:?}", inp.0.0);
    context_data_qry.iter().collect()
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


fn example_consideration_one(
    _inputs: types::ConsiderationInputs,
    qry: Query<&ActionTrackerState>
) -> types::ConsiderationOutputs {
    let mut good_cnt = 0;
    let mut bad_cnt = 0;

    for tracker in qry {
        match tracker.get_state() {
            ActionState::Failed => { bad_cnt += 1 },
            _ => { good_cnt += 1 },
        }
    }

    let total_cnt = good_cnt + bad_cnt;

    let val = if total_cnt > 0 {
        (good_cnt as ActionScore) / (total_cnt as ActionScore)
    } else {
        1.
    };

    val.into()
}

/// Trivial Consideration, returns a flat value.
fn example_consideration_two(
    _inputs: types::ConsiderationInputs,
) -> types::ConsiderationOutputs {
    0.9.into()
}

/// A Consideration that measures the distance (Euclidian, so straight-line) between 
/// the AI's Pawn and the Context 'target'. 
/// 
/// This is a common and insanely useful building block for a variety of Actions, e.g. 
/// - To favor targets close by for pickup/interaction/combat 
/// - To select spawn locations a minimum distance away from the player
/// - To cheaply pre-filter locations for pathfinding or raycasting Considerations downstream
fn example_consideration_three(
    inputs: types::ConsiderationInputs,
    qry: Query<&Position2d>
) -> ConsiderationOutputs {
    
    let (ai, maybe_pawn, targ) = inputs.0;

    let pawn = match maybe_pawn {
        None => {
            bevy::log::error!(
                "example_consideration_three requires a Pawn, but AI {:?} does not have one!",
                ai, 
            );
            return None
        },
        Some(p) => p,
    };

    let pawn_pos = match qry.get(pawn) {
        Err(_) => {
            bevy::log::error!(
                "example_consideration_three requires the Pawn to have a Position2d, but Pawn {:?} of AI {:?} does not!",
                pawn, ai,
            );
            return None
        }
        Ok(pos) => pos,
    };

    let targ_pos = match qry.get(targ) {
        Err(_) => {
            bevy::log::error!(
                "example_consideration_three requires the Context to have a Position2d, but Context {:?} for AI {:?} does not!",
                targ, ai,
            );
            return None
        }
        Ok(pos) => pos,
    };

    // Calculate the actual raw score:
    let val = pawn_pos.euclid_distance(targ_pos);

    // We'll use .into() to ensure that we can ignore simple output interface changes.
    val.into()
}


fn main() {
    let mut app = App::new();

    // A custom user-defined Curve
    let leaky_curve = LinearCurve.hard_leak(0.5);

    app
    // Enables the main Cortex integration:
    .add_plugins(CortexPlugin)
    
    // Configures the app to shut down once all Actions are finished at the end of a tick, plus logs and such:
    .add_plugins(CortexTestPlugin)

    // Registering Considerations/ContextFetchers/Curves.
    //
    // We'll use an 'e2e::' prefix as a convention to quasi-namespace user-registered Systems. 
    // This is not required, just a good practice to avoid key collisions from multiple Plugins. 
    // In case of a collision, the most recently registered value takes precedence. 
    .register_consideration(example_consideration_one, "e2e::One")
    .register_consideration(example_consideration_two, "e2e::Two")
    .register_consideration(example_consideration_three, "e2e::DistanceToPawn2d")
    .register_context_fetcher(example_context_fetcher, EXAMPLE_CONTEXT_FETCHER_NAME)
    .register_utility_curve(leaky_curve, "e2e::Linear50pHardLeak")
    
    // Setting up various demo Entities
    .add_systems(Startup, (
        setup_example_context, 
        setup_example_entity, 
        setup_default_action_tracker_config,
    ))

    // Handling Actions
    .add_systems(FixedUpdate, (
        example_action_tracker_handler,
    ).chain())
    .add_observer(example_action)
    ;

    app.run();
}
