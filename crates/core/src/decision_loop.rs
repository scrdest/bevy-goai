use std::collections::HashMap;
use bevy::prelude::*;
use crate::actions::{*};
use crate::context_fetchers::{ContextFetcherRequest, ContextFetchResponse};
use crate::considerations::{ConsiderationRequest, ConsiderationResponse};
use crate::events::AiDecisionRequested;
use crate::smart_object::ActionSetStore;
use crate::types::{self, ActionScore};
use crate::utility_concepts::{ConsiderationIdentifier};


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
pub fn ai_action_gather_phase(
    event: On<AiDecisionRequested>,
    actionset_store: Res<ActionSetStore>,
    mut request_writer: MessageWriter<ContextFetcherRequest>,
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
                ContextFetcherRequest::new(
                    entity,
                    action_spec.clone(),
                )
            );
        }
    }
}


pub fn ai_action_prescoring_phase(
    mut reader: MessageReader<ContextFetchResponse>,
    mut request_writer: MessageWriter<ConsiderationRequest>,
    // To clear out stale entries in preparation for main scoring:
    mut best_scores: ResMut<crate::action_runtime::BestScoringCandidateTracker>,
    // We need this for a very specific edge-case, see below:
    mut response_writer: MessageWriter<ConsiderationResponse>,
) {
    bevy::log::debug_once!("ai_action_prescoring_phase: Triggered ai_action_prescoring_phase()");

    // Clear the scores to avoid hangovers between runs.
    best_scores.current_winner.clear();

    for (msg, msg_id) in reader.read_with_id() {
        bevy::log::debug!("ai_action_prescoring_phase: Processing CF message {:?} (ID: {:?})", msg, msg_id);

        let audience = &msg.audience;
        let action_template = &msg.action_template;
        let considerations = &action_template.considerations;
        let contexts = &msg.contexts;

        for ctx in contexts {
            bevy::log::debug!("ai_action_prescoring_phase: Scoring context for template {:?}: {:#?}", action_template.name, ctx);

            if considerations.is_empty() {
                // Special case - an ActionTemplate with NO Considerations always returns max score.
                // Effectively means that an Action is always available. 
                // Likely niche, but good QoL for designers in my experience.
                response_writer.write(ConsiderationResponse { 
                    name: ConsiderationIdentifier::from("<no Considerations>".to_string()), 
                    entity: *audience,
                    scored_action_template: action_template.to_owned(), 
                    scored_context: ctx.to_owned(), 
                    score: 1. 
                });
                continue;
            }

            let request_batch = considerations.iter().map(|consdata| {
                ConsiderationRequest {
                    entity: *audience,
                    scored_action_template: action_template.to_owned(),
                    scored_context: ctx.to_owned(),
                    consideration_key: consdata.func_name.to_owned(),
                    curve_key: consdata.curve_name.to_owned(),
                    min: consdata.min,
                    max: consdata.max,
                }
            });
            request_writer.write_batch(request_batch);

            // let num_considerations = considerations.len();
            // if num_considerations > 0 {
            //     // Correction formula as per GDC 2015 "Building a Better Centaur AI" 
            //     //   presentation by Dave Mark and Mike Lewis.
            //     // Ensures that we do not penalize Actions for having multiple Considerations.

            //     let floaty_num_considerations = num_considerations as f32;
            //     let modification_factor = 1. - (1. / floaty_num_considerations);
            //     let makeup_val = (1. - curr_score) * modification_factor;
            //     let adjusted_score = curr_score + (makeup_val * curr_score);

            //     curr_score = adjusted_score
            // }
            
            // bevy::log::debug!("ai_action_prescoring_phase: Scored context for template {:?}: {:#?} => score={:?}, best={:?} ignored={:?}", action_template.name, ctx, curr_score, best_score, ignored);
        }
    }
}


pub fn ai_action_scoring_phase(
    mut commands: Commands,
    mut best_scores: ResMut<crate::action_runtime::BestScoringCandidateTracker>,
    mut consideration_reader: MessageReader<ConsiderationResponse>,
) {
    let ai_to_best_score = &mut best_scores.current_winner;
    let mut score_map: HashMap<Entity, (ActionScore, &ActionTemplate, &ActionContext)> = HashMap::new();
    // let mut ai_and_action_to_consideration_hits: HashMap<(&ActionTemplate, &ActionContext), usize> = HashMap::new();

    for consideration_resp in consideration_reader.read() {
        bevy::log::debug!(
            "ai_action_scoring_phase: Scored Consideration {:?} for Context {:?} for ActionTemplate{:#?} => score={:?}", 
            consideration_resp.name, 
            consideration_resp.scored_context, 
            consideration_resp.scored_action_template.name, 
            consideration_resp.score,
        );
        let entity = consideration_resp.entity;
        let action_template = &consideration_resp.scored_action_template;
        let action_ctx = &consideration_resp.scored_context;

        let mut curr_raw_score = score_map.get(&entity).map(
            |trip| trip.0
        ).unwrap_or(1.);
        let raw_consideration_score = consideration_resp.score;
        curr_raw_score *= raw_consideration_score;

        score_map.insert(entity, (curr_raw_score, action_template, action_ctx));
    }

    for (entity, (curr_raw_score, action_template, action_ctx)) in score_map.iter() {
        let maybe_best_for_ai = ai_to_best_score.get(&entity);

        let current_is_better = match maybe_best_for_ai {
            None => true,
            Some(best_for_ai) => {
                match best_for_ai {
                    None => true,
                    Some(bestcand) => {
                        let true_score = curr_raw_score * action_template.priority;
                        let curr_best = bestcand.0;
                        true_score > curr_best
                    }
                }
            }
        };

        if current_is_better {
            ai_to_best_score.insert(
                *entity, 
                Some((
                    curr_raw_score * action_template.priority,
                    (*action_template).to_owned(),
                    (*action_ctx).to_owned(),
                ))
            );
        }
    }

    for (entity_id, maybe_best_triple) in best_scores.current_winner.iter() {
        maybe_best_triple.iter().for_each(
            |best_triple| {
                let (
                    best_score, 
                    best_act, 
                    best_ctx
                ) = best_triple;

                commands.trigger(crate::events::AiActionPicked::new(
                    entity_id.to_owned(),
                    best_act.action_key.to_owned(),
                    best_act.name.to_owned(),
                    best_ctx.to_owned(),
                    best_score.to_owned(),
                ));
            }
        );
    }
}

// #[derive(Event)]
// pub(crate) struct TriggerAiActionScoringPhase;

// pub(crate) fn ai_action_prescoring_phase_observer(
//     _trigger: On<TriggerAiActionScoringPhase>,
//     reader: MessageReader<ContextFetchResponse>,
//     writer: MessageWriter<ConsiderationRequest>,
//     best_scores: ResMut<crate::action_runtime::BestScoringCandidateTracker>,
//     response_writer: MessageWriter<ConsiderationResponse>,
// ) {
//     ai_action_prescoring_phase(reader, writer, best_scores, response_writer);
// }

// pub(crate) fn ai_action_prescoring_phase_observer_trigger_system(
//     mut commands: Commands,
// ) {
//     commands.trigger(TriggerAiActionScoringPhase);
// }
