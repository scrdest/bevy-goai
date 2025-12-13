use std::borrow::Borrow;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::reflect::{func::ArgList};
use crate::actions::{*};
use crate::events::AiDecisionRequested;
use crate::smart_object::ActionSetStore;
use crate::types::{self, ActionKey, Context};


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

        bevy::log::debug!("decision_process: Available actions for {:?} are: {:#?}", entity, smartobjects.actionset_refs);
        
        for action_spec in available_actions {
            bevy::log::debug!("decision_process: {:?}: Evaluating actionspec {:?}", entity, action_spec.name);
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
            bevy::log::debug!("decision_process: {:?}: Best action is {:?}", entity, scored_action.action.name);
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


/// A Message representing a single request for a ContextFetcher call from the AI lib to user code. 
/// 
/// Users are expected to implement a System that uses a MessageReader for this type and dispatches 
/// their custom logic to handle them on a case-by-case basis..
#[derive(Message, Debug)]
pub struct ContextFetcherLibraryRequest {
    pub action_template: ActionTemplate,
    pub audience: Entity, 
}

impl ContextFetcherLibraryRequest {
    fn new(
        audience: Entity, 
        action_template: ActionTemplate,
    ) -> Self {
        Self {
            action_template: action_template,
            audience: audience,
        }
    }
}

#[derive(Message, Debug)]
pub struct ContextFetchResponse {
    /// The meat of the response - the Context that has been requested.
    contexts: types::ContextList, 
    
    /// The ActionTemplate this request came for (mainly to tie it back together as an Action)
    action_template: ActionTemplate, 

    /// The AI this was requested for; primarily so that we can split 
    /// the scoring process per each Audience, 
    /// even if the Messages for them wind up interleaved.
    audience: Entity, 
}

impl ContextFetchResponse {
    pub fn new(
        action_template: ActionTemplate,
        contexts: types::ContextList,
        audience: Entity,
    ) -> Self {
        Self {
            action_template: action_template,
            contexts: contexts,
            audience: audience,
        }
    }
}

/// Stage 1 of an AI decision loop. 
/// 
/// Iterate through all available ActionTemplates (from all available sources) 
/// and turn them into actual Actions for evaluation.
/// 
/// As a reminder an Action = ActionTemplate + Context.
/// 
/// Contexts are provided by what this library calls ContextFetchers - Bevy Systems 
/// that make World queries to figure out sets of potential parameters (i.e. Contexts) for an Action.
/// 
/// This may be finding promising positions to pathfind to, enemies in LOS to attack, allies to help, 
/// or anything at all really - the concept is fully generic.
/// 
/// ContextFetchers are user-defined Systems; the library cannot know upfront what logic they will run, 
/// since it is, by design, arbitrary - you should be able to run any World Query you want in your apps.
/// 
/// We achieve that by message-passing - the AI Engine asks the App to provide Contexts by writing a 
/// Message, the App responds with another Message with them, neither needs to share the implementation 
/// details with the other.
/// 
/// As a bonus, that also means the code is more or less compatible across different implementations! 
/// 
/// Even if the AI requests a CF that no longer exists, it will simply ignore the associated Action as 
/// a possibility (which may mean the AI is a bit silly, but the app as a whole can continue operating)!
pub(crate) fn ai_action_gather_phase(
    event: On<AiDecisionRequested>,
    actionset_store: Res<ActionSetStore>,
    mut request_writer: If<MessageWriter<ContextFetcherLibraryRequest>>,
) {
    let entity = event.event_target();
    let maybe_smartobjects = &event.smart_objects;
    
    // 1. Gather ActionSets from Smart Objects
    
    // Early termination - we have no real options in this case => idle.
    // Note that there is no notion of available Actions *NOT* tied to a SO; at
    // minimum, you'd have a SO with the key representing *the Controller itself*.
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

        bevy::log::debug!("ai_action_gather_phase: Available actions for {:?} are: {:#?}", entity, smartobjects.actionset_refs);

        // 2. Emit a request for Context for each ActionTemplate.
        
        for action_spec in available_actions {
            bevy::log::debug!("ai_action_gather_phase: AI {:?}: Requesting Contexts for actionspec {:?}", entity, action_spec.name);
            
            request_writer.write(
                ContextFetcherLibraryRequest::new(
                    entity,
                    action_spec.clone(),
                )
            );
        }
    }
}


pub(crate) fn ai_action_scoring_phase(
    app_registry: Res<AppFunctionRegistry>,
    mut commands: Commands,
    mut reader: MessageReader<ContextFetchResponse>,
) {
    bevy::log::debug_once!("ai_action_scoring_phase: Triggered ai_action_scoring_phase()");
    let registry = app_registry.read();

    // The ContextFetchResponse buffer is not split by source AI. We need to split them up.
    let mut ai_to_best_map: HashMap<Entity, Option<(f32, &ActionTemplate, &Context)>> = HashMap::new();

    for (msg, msg_id) in reader.read_with_id() {
        bevy::log::debug!("ai_action_scoring_phase: Processing CF message {:?} (ID: {:?})", msg, msg_id);

        let audience = &msg.audience;

        let best_tuple = ai_to_best_map
            .get(audience)
            .cloned()
            .unwrap_or_default()
        ;

        let best_score = best_tuple
            .map(|tup| tup.0)
            .unwrap_or(0.)
        ;


        let best_ctx = best_tuple
            .and_then(|tup| Some(tup.1))
        ;

        let action_template = &msg.action_template;
        let considerations = &action_template.considerations;
        let contexts = &msg.contexts;

        let callable_considerations: Vec<RunnableConsideration> = considerations.iter().map(
            |consdata| {
                let func = action_template.resolve_consideration(&consdata.func_name.borrow(), &registry);
                let curve = action_template.resolve_curve(&consdata.curve_name.borrow(), &registry);
                let min = consdata.min;
                let max = consdata.max;
                RunnableConsideration {
                    func,
                    curve,
                    min,
                    max,
                }
            }
        ).collect();

        for ctx in contexts {
            bevy::log::debug!("ai_action_scoring_phase: Scoring context for template {:?}: {:#?}", action_template.name, ctx);
            
            let mut curr_score: f32 = 1.;
            let mut ignored: bool = false;

            for consideration in callable_considerations.iter() {
                let args = ArgList::new()
                    .with_ref(ctx)
                ;
                let dyn_score = consideration.func.call(args).unwrap().unwrap_owned();
                let cast_score = dyn_score.try_take();

                let score: f32 = cast_score.unwrap_or(0.);
                curr_score *= score;

                // Early termination; it's not gonna be worth it.
                if curr_score <= best_score {
                    // Note that there may be some misbehavior here, as
                    // we are not accounting for the makeup factor later.
                    ignored = true;
                    break;
                };
            }
            
            if ignored { 
                // break inner loop, skip the whole context - it's no bueno
                bevy::log::debug!("ai_action_scoring_phase: Scored context for template {:?}: {:#?} => score={:?}, best={:?} ignored={:?}", action_template.name, ctx, curr_score, best_score, ignored);
                continue 
            };

            let num_considerations = considerations.len();
            if num_considerations > 0 {
                // Correction formula as per GDC 2015 "Building a Better Centaur AI" 
                //   presentation by Dave Mark and Mike Lewis.
                // Ensures that we do not penalize Actions for having multiple Considerations.

                let floaty_num_considerations = num_considerations as f32;
                let modification_factor = 1. - (1. / floaty_num_considerations);
                let makeup_val = (1. - curr_score) * modification_factor;
                let adjusted_score = curr_score + (makeup_val * curr_score);

                curr_score = adjusted_score
            }
            
            bevy::log::debug!("ai_action_scoring_phase: Scored context for template {:?}: {:#?} => score={:?}, best={:?} ignored={:?}", action_template.name, ctx, curr_score, best_score, ignored);

            if best_ctx.is_none() || (curr_score > best_score) {
                // Update the best score in this round to make sure the remaining stuff beats it.
                ai_to_best_map.insert(*audience, Some((curr_score, action_template, ctx)));
            };
        }
    }

    for (entity_id, maybe_best_tup) in ai_to_best_map {
        if let Some(best_tup)= maybe_best_tup {
            let best_score = best_tup.0;
            let best_action = best_tup.1;
            let best_context = best_tup.2;

            // raise an event for each AI with the highest scoring Action
            commands.trigger(crate::events::AiActionPicked::new(
                entity_id,
                best_action.action_key.to_owned(),
                best_action.name.to_owned(),
                best_context.to_owned(), 
                best_score,
            ));
        }
    }
}

#[derive(Event)]
pub(crate) struct TriggerAiActionScoringPhase;

pub(crate) fn ai_action_scoring_phase_observer(
    _trigger: On<TriggerAiActionScoringPhase>,
    app_registry: Res<AppFunctionRegistry>,
    mut commands: Commands,
    mut reader: MessageReader<ContextFetchResponse>,
) {
    ai_action_scoring_phase(app_registry, commands, reader);
}

pub(crate) fn ai_action_scoring_phase_observer_trigger_system(
    mut commands: Commands,
) {
    commands.trigger(TriggerAiActionScoringPhase);
}
