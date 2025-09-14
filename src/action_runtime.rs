use std::collections::HashMap;

use bevy::ecs::reflect::ReflectCommandExt;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::actions::ScoredAction;
use crate::ai::AIController;
use crate::events::GoaiActionEvent;
use crate::smart_object::{SmartObjects, ActionSetStore};

/* Experimental nonsense, remove me */
use crate::actions::ActionContext;
use crate::events::{ActionEventFactory, ActionEvent};

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


#[derive(Component, Default, Debug, Reflect)]
#[reflect(Component)]
struct TestActionEventFactory {
    queue: Vec<ActionContext>
}

impl TestActionEventFactory {
    fn push_context(&mut self, ctx: ActionContext) {
        self.queue.push(ctx)
    }
}

impl ActionEventFactory for TestActionEventFactory {
    type AsEvent = TestActionEvent;

    fn get_input_contexts(&mut self) -> Vec<ActionContext> {
        let out = self.queue.to_vec();
        self.queue = Vec::new();
        out
    }

    fn to_action_event(&self, ctx: &ActionContext) -> Self::AsEvent {
        TestActionEvent { 
            ctx: ctx.to_owned(), 
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


/// The heart of the AI system - the system that actually decides what gets done.
pub(crate) fn decision_loop(
    query: Query<(Entity, &AIController, Option<&SmartObjects>, Option<&CurrentAction>)>,
    mut commands: ParallelCommands,
    actionset_store: Res<ActionSetStore>,
    function_registry: Res<AppFunctionRegistry>,
) {
    query.par_iter().for_each(
        |(entity, ai, maybe_smartobjects, maybe_current_action)|
    {
        bevy::log::debug!("Current action for {:?} is {:?}", entity, maybe_current_action);

        if maybe_current_action.is_none() {
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

                
                let best_action = best_action;

                // 3. Trigger best action execution (raise event)
                if let Some(scored_action) = best_action {
                    bevy::log::debug!("{:?}: Best action is {:?}", entity, scored_action.action.name);
                    let new_current = scored_action.to_owned();
                    
                    let mut factory = TestActionEventFactory::default();
                    factory.push_context(scored_action.action.context);
                    let refl_factory = factory.to_dynamic();

                    commands.command_scope(|mut cmds| {
                        cmds.entity(entity).insert(CurrentAction {
                            action: new_current,
                            state: ActionState::Running,
                        });

                        cmds.entity(entity).insert_reflect(refl_factory);
                        // cmds.entity(entity).insert(
                        //     factory
                        // );
                    });
                }
            }
        }
    }
})
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use serde_json;
    use crate::actions::ActionTemplate;
    use crate::actionset::ActionSet;
    use crate::arg_values::ContextValue;
    use crate::utility_concepts::ContextFetcherIdentifier;
    use super::*;

    const TEST_CONTEXT_FETCHER_NAME: &str = "TestCF";

    fn test_action(
        trigger: Trigger<TestActionEvent>, 
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
                event: 1,
            }
        ];

        let test_actionset = ActionSet {
            name: "TestActionSet".to_string(),
            actions: Vec::from(test_actions)
        };

        actionset_store.map_by_name.insert(test_actionset.name.to_owned(), test_actionset);

        commands.spawn((
            AIController::default(),
            SmartObjects {
                actionset_refs: Vec::from(["TestActionSet".to_string()])
            }
        ));
    }

    fn raise_action_events<
        AEF: ActionEventFactory<AsEvent: std::fmt::Debug>
        + Component<Mutability = bevy::ecs::component::Mutable>
    > (
        mut event_factories: Query<&mut AEF>,
        commands: ParallelCommands,
    ) {
        bevy::log::debug!("Running raise_action_events()...");
        
        event_factories.par_iter_mut().for_each(|mut factory| {
            let events = factory.run();
            bevy::log::debug!("raise_action_events(): Collected events: {:?}", events);

            commands.command_scope(|mut cmds| {
                events.into_iter().for_each(|evt| {
                    cmds.trigger(evt)
                });
            });
        });
    }


    #[test]
    fn test_run_action() {
        let mut app = App::new();

        app
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1000))),
            LogPlugin { 
                level: bevy::log::Level::DEBUG, 
                custom_layer: |_| None, 
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
            }
        ))
        .init_resource::<ActionSetStore>()
        .register_type::<TestActionEventFactory>()
        .register_function_with_name(TEST_CONTEXT_FETCHER_NAME, test_context_fetcher)
        .add_systems(Startup, setup_test_entity)
        .add_systems(FixedUpdate, (
            decision_loop, 
            raise_action_events::<TestActionEventFactory>
        ))
        .add_observer(test_action)
        ;

        app.run();
    }
}


