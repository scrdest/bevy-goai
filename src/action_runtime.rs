use std::borrow::Borrow;
use bevy::prelude::*;
use bevy::reflect::func::ArgList;
use serde::{Deserialize, Serialize};
use crate::actions::{ScoredAction};
use crate::ai::{AIController};
use crate::pawn::Pawn;

#[derive(Reflect, Serialize, Deserialize, Debug)]
pub enum ActionState {
    Running,
    Succeeded,
    Failed,
}

// Action Execution
#[derive(Component)]
pub struct CurrentAction {
    /// A Component that represents the Action selected for execution by a specific AI entity.
    /// The execution is a fairly simple System - we simply run each Action's function and update the state.
    action: ScoredAction,
    state: ActionState, 
}

fn run_actions(
    actions: Query<(&CurrentAction, &AIController, Option<&Pawn>)>,
    fn_registry: Res<AppFunctionRegistry>,
) {
    let fn_registry_reader = fn_registry.read();

    for (current_action, controller, maybe_pawn) in actions.iter() {
        let action_fn_name = &current_action.action.action.func;
        let lookup = fn_registry_reader.get(action_fn_name.borrow());
        if let None = lookup {
            println!("Function lookup for action {:?} failed!", action_fn_name);
            continue
        };
        let action_fn_dyn = lookup.unwrap();
        let ctx = &current_action.action.action.context;
        let args = 
            ArgList::new()
            .with_ref(ctx)
            .with_ref(&current_action.state)
        ;
        let result = action_fn_dyn.call(args);
        if let Err(err) = result {
            println!("Function {:?} returned an error: {:?}", action_fn_name, err);
            continue
        };
        let new_state_raw = result.unwrap();
        let new_state: ActionState = new_state_raw.unwrap_owned().try_take().unwrap();
        // current_action.state = new_state;
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;
    use crate::actions::{Action, DynFuncName};
    use crate::arg_values::ContextValue;
    use crate::type_registry::{TypeRegistryIdentifier, ReflectTypeRegistry};
    use serde_json;

    fn test_action(
        ctx: &HashMap<String, ContextValue>, 
        state: &ActionState
    ) -> ActionState {

        let json_state = serde_json::ser::to_string(state);
        let state_name = json_state.unwrap();
        println!("Current state is {}", state_name);

        let context_mapping = ctx.get(&state_name);

        let new: ActionState = match context_mapping {
            None => None,
            Some(cv) => {
                let clone_val = cv.clone();
                let cvstring: String = clone_val.try_into().unwrap();
                let unjsond = serde_json::de::from_str(&cvstring).unwrap();
                println!("Current unjsond is {:?}", unjsond);
                unjsond
            }
        }.unwrap();

        println!("New state is {:?}", new);
        new
    }

    fn setup_test_entity(
        mut commands: Commands,
        func_registry: Res<AppFunctionRegistry>,
    ) {
        let testfunc_name = DynFuncName::from_string_identifier(
            "testfunc".to_string(), 
            &ReflectTypeRegistry::AppFunc(func_registry),
        ).unwrap();

        let mut context: HashMap<String, ContextValue> = HashMap::with_capacity(2);
        // As an artifact of how we use JSON serde, we need to add escaped quotes around strings here.
        context.insert("\"Running\"".to_string(), "\"Failed\"".to_string().into());
        context.insert("\"Failed\"".to_string(), "\"Failed\"".to_string().into());
        let context = context; // lock out mutability

        let action = ScoredAction {
            action: Action {
                name: "testAction".to_string(),
                func: testfunc_name,
                context,
            },
            score: 1.0
        };

        commands.spawn((
            AIController::default(),
            CurrentAction {
                action,
                state: ActionState::Running
             }
        ));
    }


    #[test]
    fn test_run_action() {
        let testfunc = test_action.into_function();

        let mut app = App::new();
        app
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once())
        )
        .register_function_with_name("testfunc", testfunc)
        .add_systems(Startup, setup_test_entity)
        .add_systems(Update, run_actions)
        .run();
    }
}


