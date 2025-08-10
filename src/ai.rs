use bevy::prelude::*;
use crate::actionset::ActionSet;
use crate::action_runtime::CurrentAction;
use crate::smart_object::SmartObject;

// AI Agent Core
// The AIController is the main 'something running AI calculations' component.
// 
#[derive(Component, Default)]
pub struct AIController {}

// fn decision_loop(
//     mut query: Query<(Entity, &AIController)>,
//     smart_objects: Query<(Entity, &SmartObject)>,
//     action_sets: Res<Assets<ActionSet>>,
//     mut commands: Commands,
// ) {
//     for (entity, mut ai) in query.iter_mut() {
//         if ai.current_action.is_none() {
//             // 1. Gather ActionSets from Smart Objects
//             let mut available_actions = Vec::new();
//             for (obj_entity, smart_obj) in smart_objects.iter() {
//                 if ai.is_in_range(obj_entity) {
//                     if let Some(action_set) = action_sets.get(&smart_obj.action_set) {
//                         available_actions.extend(action_set.actions.clone());
//                     }
//                 }
//             }

//             // 2. Score Actions
//             let mut best_score = -1.0;
//             let mut best_action = None;
            
//             for action_spec in available_actions {
//                 let contexts = (CONTEXT_FETCHER_REGISTRY.get(&action_spec.context_fetcher))(&ai.world);
                
//                 for context in contexts {
//                     let mut score = 1.0;
                    
//                     for consideration in &action_spec.considerations {
//                         let input = (CONSIDERATION_FUNCTIONS.get(&consideration.function))(
//                             &ai.brain,
//                             &context,
//                             &consideration.static_args,
//                         );
//                         let normalized = consideration.curve.normalize(input, consideration.min, consideration.max);
//                         score *= normalized;
                        
//                         // Early termination
//                         if score < best_score * 0.5 { break; }
//                     }
                    
//                     if score > best_score {
//                         best_score = score;
//                         best_action = Some((action_spec.template.clone(), context));
//                     }
//                 }
//             }

//             // 3. Execute best action
//             if let Some((template, context)) = best_action {
//                 commands.entity(entity).insert(CurrentAction {
//                     template,
//                     context,
//                     state: ActionState::Running,
//                 });
//             }
//         }
//     }
// }
