use std::collections::HashMap;

use bevy::ecs::reflect::ReflectCommandExt;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::actions::ScoredAction;
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


#[derive(Reflect, Serialize, Deserialize, Debug, Clone)]
pub enum ActionState {
    Running,
    Succeeded,
    Failed,
}

// Action Execution

/// A Component that represents the Action selected for execution by a specific AI entity.
#[derive(Component, Debug)]
pub struct CurrentAction {
    action: ScoredAction,
    state: ActionState, 
}


/// Supporting Event for triggering a decision_process() for an AI.
/// Raised whenever an active AI starts a tick without an Action.
/// Should generally not be raised more than once per Entity per tick.
#[derive(EntityEvent)]
pub struct AiDecisionRequested {
    entity: Entity,
    smart_objects: Option<SmartObjects>,
}

/// An Event that signals the decision engine picked the new best Action
/// and provides details about the chosen Action (abstract ID, context).
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
/// This is the key code that makes this a Utility AI.
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


