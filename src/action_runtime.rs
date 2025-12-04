use std::collections::HashMap;

use bevy::ecs::reflect::ReflectCommandExt;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::actions::{Action, ScoredAction};
use crate::ai::AIController;
use crate::smart_object::{SmartObjects, ActionSetStore};

/* Experimental nonsense, remove me */
use crate::actions::ActionContext;
use crate::events::{ActionEvent};

#[derive(Debug, Event)]
struct TestActionEvent{
    ctx: ActionContext,
    state: ActionState,
}

impl Default for TestActionEvent {
    fn default() -> Self {
        Self { ctx: Default::default(), state: ActionState::Running }
    }
}

impl ActionEvent for TestActionEvent {
    fn from_context(context: ActionContext) -> Self {
        Self {
            ctx: context,
            state: ActionState::Running,
        }
    }
}
/* END EXPERIMENTAL NONSENSE */


/// Supporting Event for triggering a decision_process() for an AI.
/// Raised whenever an active AI starts a tick without an Action.
/// 
/// Should generally NOT be raised more than once per Entity per tick 
/// or you are likely running the same calculation multiple times.
#[derive(EntityEvent)]
pub struct AiDecisionRequested {
    entity: Entity,
    smart_objects: Option<SmartObjects>,
}

/// An Event that signals the decision engine picked the new best Action
/// and provides details about the chosen Action (abstract ID, context).
/// 
/// Primarily expected to be raised by the decision_process() System 
/// and listened to by consumers for remapping into more Action-specific logic
/// (e.g. raising an Event for a *specific* Action implementation).
#[derive(Event, Debug)]
pub struct AiActionPicked {
    /// Identifier for the handling event (e.g. "GoTo"). 
    /// This is effectively a link to the *implementation* of the action. 
    pub action_key: String,

    /// Human-readable primary identifier; one action_key may handle distinct action_names 
    /// (e.g. action_key "GoTo" may cover action_names "Walk", "Run", "Flee", etc.).
    /// In other words, this is what this action represents *semantically*, and is less likely
    /// to change for technical purposes.
    pub action_name: String,

    pub context: ActionContext,

    /// The AI that requested this Action. 
    pub source_ai: Entity,
}

/// The heart of the AI system - the system that actually decides what gets done.
/// This is the key code that makes this a IAUS Utility AI.
/// 
/// The logic here is fundamentally not that complex; we are simply mapping over all 
/// ActionTemplates from all SmartObjects we have access to, gathering all available Contexts
/// for those ActionTemplates, scoring all the (ActionTemplate, Context) pairs, and picking the winner.
/// 
/// There IS some slight wizardry in how exactly the scoring works, optimization, and other minutia, 
/// but the core algorithm is a greedy heuristic search with a depth of one; 
/// basic A* pathfinding is already far more sophisticated than this, but it works, and fast!
pub(crate) fn decision_process(
    event: On<AiDecisionRequested>,
    mut commands: Commands,
    actionset_store: Res<ActionSetStore>,
    function_registry: Res<AppFunctionRegistry>,
) {
    let entity = event.entity;
    let maybe_smartobjects = &event.smart_objects;
    
    // 1. Gather ActionSets from Smart Objects
    
    // Early termination - we have no real options in this case => idle.
    // Note that there is no notion of available Actions *NOT* tied to a SO; at
    // minimum, you'd have a SO with the key representing the Controller itself.
    if let Some(smartobjects) = maybe_smartobjects {
        let available_actions = smartobjects.actionset_refs.iter().filter_map(
            |actionset_key| {
                let maybe_act = actionset_store.map_by_name.get(actionset_key);
                maybe_act
            }
        )
        .flat_map(|acts| {
            acts.actions.to_vec()
        });

        // 2. Score Actions
        let mut best_score = 0.0;
        let mut best_action: Option<ScoredAction> = None;

        bevy::log::debug!("Available actions for {:?} are: {:#?}", entity, smartobjects.actionset_refs);
        
        for action_spec in available_actions {
            bevy::log::debug!("{:?}: Evaluating actionspec {:?}", entity, action_spec.name);
            let best_scoring_combo = action_spec.run_considerations(&function_registry.read(), Some(best_score));
            if best_scoring_combo.is_none() {
                continue
            }

            let best_scoring_combo = best_scoring_combo.unwrap();

            // if we got here, we know RHS >= LHS, otherwise it would have not been a Some<T>
            best_score = best_scoring_combo.score;
            best_action = Some(best_scoring_combo);
        }
        
        let best_action = best_action;

        // 3. Trigger best action execution (raise event)
        if let Some(scored_action) = best_action {
            bevy::log::debug!("{:?}: Best action is {:?}", entity, scored_action.action.name);
            let new_current = scored_action.to_owned();

            commands.trigger(AiActionPicked {
                action_name: new_current.action.name,
                action_key: new_current.action.action_key,
                context: new_current.action.context, 
                source_ai: entity,
            });
        }
    }
}


#[derive(Reflect, Serialize, Deserialize, Debug, Clone)]
pub enum ActionState {
    // Initial states:
    Queued, // Planned but not started and cannot start yet - waiting on something (likely other Actions).
    Ready, // Can start now, hadn't done literally anything yet though.
    
    // Progressed states: 
    Running, // Started but didn't finish yet, will continue.
    Paused, // Started and can continue, but put on hold for now.
    
    // Terminal states:
    Succeeded,  // Did all it was supposed to and is no longer needed :D
    Failed,  // We gave up due to getting stuck/timeout/etc. :c
    Cancelled,  // We gave up because a player decided we should :I
}

impl ActionState {
    /// A shorthand for checking if an Action is in one of the Initial states (e.g. Ready).
    fn is_initial(&self) -> bool {
        match self {
            Self::Queued => true,
            Self::Ready => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action is in one of the Progressed states (e.g. Running).
    fn is_progressed(&self) -> bool {
        match self {
            Self::Running => true,
            Self::Paused => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action is in one of the Terminal states (e.g. Succeeded).
    fn is_terminal(&self) -> bool {
        match self {
            Self::Succeeded => true,
            Self::Failed => true,
            Self::Cancelled => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action should be processed/'ticked'. If false, we can skip it.
    fn should_process(&self) -> bool {
        match self {
            Self::Ready => true,
            Self::Running => true,
            _ => false,
        }
    }
}


// Action Execution
/// A Component that makes this Entity track an AI Action, i.e. store and expose 
/// data about some Action's execution (e.g. 'Is this still running?' or 'How long ago did it start?') 
/// across (potentially) multiple frames and/or serde operations.
/// 
/// This is the 'master'/'root' marker for tracking Actions - all code within this library will assume 
/// anything without this Component does not track anything, and anything that has any of the extension
/// Component also has this Component.
#[derive(Component, Debug)]
pub struct ActionTracker(ScoredAction);


/// An 'extension' Component for ActionTracker Bundles.
/// 
/// Adds Action state tracking to the ActionTracker.
/// In this context, state is roughly the 'lifecycle' of an Action: 
/// Pending -> Running -> Terminal (Successful/Failed/Cancelled/etc.).
/// 
/// This is a very handy piece of information. 
/// You can query it for your Actions to do setup work for Pending or 
/// skip processing for Terminal states, in your UI code to visualize
/// the state of the current behavior or provide a 'Cancel Action' button...
/// You might also (correctly) guess it powers some of the Action lifecycle Events.
#[derive(Component, Debug)]
pub struct ActionTrackerState(ActionState);

/// Helper; wraps how we store time for tracking Action runtime timining.
#[derive(Debug)]
pub enum TimeInstantActionTracker {
    Virtual(std::time::Duration),
    Real(std::time::Duration),
    VirtualAndReal((std::time::Duration, std::time::Duration))
}

/// An 'extension' Component for ActionTracker Bundles.
/// 
/// Adds Action time metadata tracking to the ActionTracker, 
/// such as the start and end times.
/// 
/// The primary purpose of this is timeouts to kill tasks that got stuck 
/// or did not get cleaned up properly for some reason and the like, but will 
/// almost certainly be handy for UIs and/or Action logic itself as well.
#[derive(Component, Debug, Default)]
pub struct ActionTrackerTimer {
    start_time: Option<TimeInstantActionTracker>,
    last_tick_time: Option<TimeInstantActionTracker>,
    end_time: Option<TimeInstantActionTracker>,
}


/// An 'extension' Component for ActionTracker Bundles.
/// 
/// Indicates that this tracker should be processed by a System that   
/// runs some sort of 'tick' logic for the associated Action. 
/// 
/// For example, a MoveTo<pos> Action might move a unit by one grid square, 
/// or one turn's worth of moves, or whatever.
/// 
/// This may be implemented as a function call, a signal emit, both, or whatever else;
/// it's entirely your call as a library user to write your Systems how you want 'em.
/// 
/// This Component is entirely optional - if you don't use it, you can just 
/// catch Events for Action start/end and handle the execution asynchronously 
/// in your game code's 'native' systems (e.g. just set a destination for your 
/// Movement system in an Observer and let it figure out the specifics itself).
#[derive(Component)]
pub struct ActionTrackerTicks;


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use serde_json;
    use crate::actions::ActionTemplate;
    use crate::actionset::ActionSet;
    use crate::arg_values::ContextValue;
    use crate::utility_concepts::ContextFetcherIdentifier;
    use super::*;

    const TEST_CONTEXT_FETCHER_NAME: &str = "TestCF";

    fn action_picked_handler(
        trigger: On<AiActionPicked>,
        mut commands: Commands,
    ) {
        // User application code - dispatches actual execution Events based on the key in the library Event.
        let event = trigger.event();
        let action_name = event.action_key.as_str();
        match action_name {
            "TestAction" => {
                commands.trigger(TestActionEvent::from_context(event.context.to_owned()));
            },
            _ => {}
        }
    }

    fn test_action(
        trigger: On<TestActionEvent>, 
    ) {
        let event = trigger.event();
        let state = &event.state;
        let maybe_ctx = Some(&event.ctx);

        let json_state = serde_json::ser::to_string(&state);
        let state_name = json_state.unwrap();
        bevy::log::debug!("Current state is {}", state_name);

        let self_name: Option<&String> = maybe_ctx.map(|ctx| ctx.get("this").unwrap().try_into().unwrap());
        bevy::log::debug!("Self name is {:?}", self_name);

        let context_mapping = maybe_ctx.map(|ctx| ctx.get(&state_name)).flatten();

        let new: ActionState = match context_mapping {
            None => None,
            Some(cv) => {
                let clone_val = cv.clone();
                let cvstring: String = clone_val.try_into().unwrap();
                let unjsond = serde_json::de::from_str(&cvstring).unwrap();
                bevy::log::debug!("Current unjsond is {:?}", unjsond);
                unjsond
            }
        }.unwrap();

        bevy::log::debug!("New state is {:?}", new);
    }

    fn test_context_fetcher() -> Vec<crate::actions::ActionContext> {
        let mut context: HashMap<String, ContextValue> = HashMap::with_capacity(3);
        // As an artifact of how we use JSON serde, we need to add escaped quotes around strings here.
        context.insert("\"Running\"".to_string(), "\"Failed\"".to_string().into());
        context.insert("\"Failed\"".to_string(), "\"Failed\"".to_string().into());
        context.insert("this".to_string(), TEST_CONTEXT_FETCHER_NAME.to_string().into());
        Vec::from([context])
    }

    fn setup_test_entity(
        mut commands: Commands,
        mut actionset_store: ResMut<ActionSetStore>,
    ) {
        let test_actions = [
            ActionTemplate  {
                name: "TestAction".to_string(),
                context_fetcher_name: ContextFetcherIdentifier(TEST_CONTEXT_FETCHER_NAME.to_string()),
                considerations: Vec::from([]),
                priority: 1.,
                action_key: "TestAction".to_string(),
            }
        ];

        let test_actionset = ActionSet {
            name: "TestActionSet".to_string(),
            actions: Vec::from(test_actions)
        };

        actionset_store.map_by_name.insert(test_actionset.name.to_owned(), test_actionset);

        let new_controller = AIController::default();
        let new_sos = SmartObjects {
            actionset_refs: Vec::from(["TestActionSet".to_string()])
        };

        let spawned = commands.spawn((
            new_controller,
        ));

        let ai_id = spawned.id();

        commands.trigger(AiDecisionRequested { 
            entity: ai_id,  
            smart_objects: Some(new_sos)
        });
    }


    #[test]
    fn test_run_action() {
        let mut app = App::new();

        app
        .add_plugins((
            // MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(200))),
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
            LogPlugin { 
                level: bevy::log::Level::DEBUG, 
                custom_layer: |_| None, 
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
                fmt_layer: |_| None,
            }
        ))
        .init_resource::<ActionSetStore>()
        .register_function_with_name(TEST_CONTEXT_FETCHER_NAME, test_context_fetcher)
        .add_observer(decision_process)
        .add_systems(Startup, setup_test_entity)
        .add_observer(action_picked_handler)
        .add_observer(test_action)
        ;

        app.run();
    }
}


