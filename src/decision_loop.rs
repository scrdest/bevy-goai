use bevy::prelude::*;
use crate::actions::{ScoredAction};
use crate::events::AiDecisionRequested;
use crate::smart_object::ActionSetStore;


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
    let entity = event.event_target();
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

            commands.trigger(crate::events::AiActionPicked::new(
                entity,
                new_current.action.action_key,
                new_current.action.name,
                new_current.action.context, 
                best_score,
            ));
        }
    }
}
