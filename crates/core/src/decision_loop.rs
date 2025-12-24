use std::collections::HashMap;
use std::rc::Rc;
use bevy::prelude::*;
use bevy::ecs::system::{SystemId, SystemState};
use crate::actions::{*};
use crate::context_fetchers::{ContextFetcherRequest, ContextFetchResponse};
use crate::considerations::{BatchedConsiderationRequest, ConsiderationMappedToSystemIds, ConsiderationResponse};
use crate::events::AiDecisionRequested;
use crate::smart_object::ActionSetStore;
use crate::types::{self, ActionScore};
use crate::utility_concepts::{ConsiderationIdentifier, CurveIdentifier};


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
    system_id_map: Res<ConsiderationKeyToSystemIdMap>,
    mut reader: MessageReader<ContextFetchResponse>,
    mut request_writer: MessageWriter<BatchedConsiderationRequest>,
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

        for (ctx_idx, ctx) in contexts.iter().enumerate() {
            bevy::log::debug!("ai_action_prescoring_phase: Scoring context for template {:?}: {:#?}", action_template.name, ctx);

            if considerations.is_empty() {
                // Special case - an ActionTemplate with NO Considerations always returns max score.
                // Effectively means that an Action is always available. 
                // Likely niche, but good QoL for designers in my experience.
                response_writer.write(ConsiderationResponse { 
                    entity: *audience,
                    scored_action_template: action_template.to_owned(), 
                    scored_context: ctx.to_owned(), 
                    score: 1. 
                });
                continue;
            }

            let system_ids = considerations.iter().map(|con| {
                let mapped = system_id_map.mapping
                    .get(&con.func_name)
                    .ok_or(())
                    .cloned()
                    ;
                
                ConsiderationMappedToSystemIds {
                    func_name: con.func_name.to_owned(),
                    consideration_systemid: mapped,
                    curve_name: con.curve_name.to_owned(),
                    min: con.min,
                    max: con.max,
                }
            });

            let request_batch = BatchedConsiderationRequest {
                entity: *audience,
                scored_action_template: action_template.to_owned(),
                scored_context: ctx.to_owned(),
                scored_context_index: (msg_id.id, ctx_idx),
                considerations: system_ids.collect(),
            };
            request_writer.write(request_batch);

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

/// Correction formula as per the GDC 2015 "Building a Better Centaur AI" 
/// presentation by Dave Mark and Mike Lewis.
/// 
/// Ensures that we do not penalize Actions for having multiple Considerations 
/// by giving scores a small bonus for each Consideration scored.
/// 
/// The input should be the raw score, the product of Consideration scores 
/// with respective Curves applied but WITHOUT the Priority multiplier on top.
/// 
/// The output is guaranteed to remain clamped within the 0-1 range; 
/// the adjustment only applies to scores below the maximum.
/// 
/// Example w/ 5 Considerations:
/// - Input 0.900 => Output = 0.972
/// - Input 0.500 => Output = 0.700
/// - Input score 1.000 => Output score = 1.000
/// 
/// Example w/ 10 Considerations:
/// - Input 0.900 => Output = 0.981
/// - Input 0.500 => Output = 0.725
/// - Input score 1.000 => Output score = 1.000
/// 
fn consideration_adjustment(
    score: types::ActionScore,
    num_considerations: usize,
) -> types::ActionScore {

    // If a score is at zero (or below, somehow), there's no saving it anyway.
    if score <= types::MIN_CONSIDERATION_SCORE {
        // We could return raw score as well, but we'll safeguard the expected range here.
        return types::MIN_CONSIDERATION_SCORE
    }

    // The formula doesn't adjust scores above 1.0 anyway; 
    // saves us some unnecessary calculations.
    if score >= types::MAX_CONSIDERATION_SCORE {
        // We could return raw score as well, but we'll safeguard the expected range here.
        return types::MAX_CONSIDERATION_SCORE
    }
    
    // Division by zero workaround.
    if num_considerations <= 0 {
        return score
    }

    let floaty_num_considerations = num_considerations as f32;
    let modification_factor = 1. - (1. / floaty_num_considerations);
    let makeup_val = (1. - score) * modification_factor;
    let adjusted_score = score + (makeup_val * score);

    adjusted_score
}


#[derive(Resource, Default)]
pub struct ConsiderationKeyToSystemIdMap {
    pub mapping: HashMap<ConsiderationIdentifier, SystemId::<(), ActionScore>>
}

/// 
pub fn ai_action_scoring_phase(
    world: &mut World,
    params: &mut SystemState<(
        MessageReader<BatchedConsiderationRequest>,
    )>
) {
    // bevy::log::debug!("Running ai_action_scoring_phase...");

    let messages: Vec<BatchedConsiderationRequest> = {
        let (
            mut request_reader, 
        ) = params.get_mut(world);

        request_reader.read().cloned().collect()
    };
    
    // Best score reached for this AI, globally
    // If any batch Score dips below this, we can discard the whole batch immediately 
    // as it cannot possibly beat the current best.
    let mut best_scoring_for_ai = HashMap::<
        Entity, (
            ActionScore, 
            Rc<ActionTemplate>, 
            (usize, usize)
        )
    >::new();

    // Best score reached for this ActionTemplate
    // This is a bit more 'local' than the per-AI score
    let mut best_scoring_template = HashMap::<
        (Entity, Rc<ActionTemplate>), 
        ActionScore
    >::new();

    let mut index_to_context_map = HashMap::<
        (Entity, (usize, usize)), 
        ActionContext
    >::new();

    // We use Rc<T> to avoid cloning data for the HashMaps
    let mut at_rc_pool: HashMap<String, Rc<ActionTemplate>> = HashMap::new();

    for msg in messages {
        bevy::log::debug!("ai_action_scoring_phase: processing {:?}", &msg);
        let ai = msg.entity;

        let curr_best_for_ai = best_scoring_for_ai
            .get(&ai)
            .map(|tup| tup.0)
            .unwrap_or(types::MIN_CONSIDERATION_SCORE)
        ;

        if curr_best_for_ai >= msg.scored_action_template.priority {
            // Priority forms a ceiling for maximum final score.
            // At Priority 1, the max score is 1.0; at 2 -> 2.0; at 5 -> 5.0 etc.
            // If we have a Priority 1 Action and the high score is 2.2, we will never beat it.
            // 
            // Note that in general, a lower-Priority template can still win over a higher-Priority one; 
            // this would happen if the high-Priority score gets cut down heavily by its Considerations.
            // 
            // For example, a P1 Idle can beat P5 Heal if IsHurt Consideration for the latter returns 0 
            // (and so the final score for Heal is 5.0 * 0.0 => 0.0)
            // 
            // Here, we are tracking the top SCORE, not top PRIORITY processed, so skipping is valid.
            continue;
        }
        
        let maybe_curr_template = at_rc_pool
            .get(&msg.scored_action_template.name)
            .cloned()
            ;

        let mut was_empty = false;
        let curr_template = maybe_curr_template.unwrap_or_else(
            || {
                was_empty = true; 
                Rc::new(msg.scored_action_template)
            }
        );
        
        if was_empty {
            at_rc_pool.insert((
                &curr_template.name).to_owned(), 
                curr_template.clone()
            );
        }
        
        let best_score_for_template = best_scoring_template
            .get(&(ai, curr_template.clone()))
        ;

        // The current total score for this AI + Action
        let mut curr_score = types::MAX_CONSIDERATION_SCORE;
        let mut consideration_count: usize = 0;

        for (cons_cnt, cons) in msg.considerations.iter().enumerate() {
            match cons.consideration_systemid {
                Err(_) => bevy::log::debug!("Failed to resolve Consideration '{:}' to a System!", cons.func_name),
                Ok(system_id) => {
                    let res = world.run_system(system_id);
                    match res {
                        Ok(positive) => bevy::log::debug!(
                            "Consideration '{:}' Score: {:?}", 
                            cons.func_name, 
                            positive
                        ),
                        Err(negative) => {
                                bevy::log::debug!(
                                "Consideration '{:}' errored: {:?}", 
                                cons.func_name, 
                                negative
                            );
                            continue;
                        }
                    }

                    let curr_template_best = best_score_for_template.copied().unwrap_or(
                        types::MIN_CONSIDERATION_SCORE
                    );

                    let raw_score = res.unwrap();
                    // todo apply curve!
                    let score = raw_score;

                    // The actual (raw) score is the product of all Consideration scores so far.
                    curr_score *= score;

                    // There is a superior Context for this ActionTemplate.
                    // We don't need to bother checking other Considerations for this Context, 
                    // as it will not get picked anyway.
                    if curr_template_best >= curr_score {
                        break;
                    }

                    // We need to know how many Considerations we have processed for later.
                    consideration_count = cons_cnt;
                }
            }
        }
        
        best_scoring_template.insert(
            (ai, curr_template.clone()), 
            // Each Context has the same amount of Considerations and same Priority, 
            // so we can store and compare raw scores without the other cruft.
            curr_score
        );

        let adjusted_score = consideration_adjustment(
            curr_score, 
            consideration_count
        );

        // todo: add a parametrizeable amount of randomness for break-evens
        let prioritized_score = adjusted_score * curr_template.priority;

        if prioritized_score > curr_best_for_ai {
            // Update frontrunners for each AI processed.
            best_scoring_for_ai.insert(
                ai, 
                (prioritized_score, curr_template, msg.scored_context_index)
            );

            // We need to be able to retrieve the actual best Context later for each AI, 
            // so we'll store the index-to-Context map for any serious candidates here.
            index_to_context_map.insert(
                (ai, msg.scored_context_index), 
                msg.scored_context
            );
        }
    }

    for (
        ai, (
            best_score, 
            best_template, 
            best_ctx_id
        )
    ) in best_scoring_for_ai {
        let best_ctx = index_to_context_map.get(
            &(ai, best_ctx_id)
        ).expect("Best-scoring ContextId is not mapped to a Context, somehow!");

        bevy::log::debug!(
            "Picking Action {:?} w/ Score {:?} for AI {:?}...", 
            &best_template.name,
            &best_score,
            &ai,
        );

        let pick_evt = crate::events::AiActionPicked {
            entity: ai,
            action_key: best_template.action_key.to_owned(),
            action_name: best_template.name.to_owned(),
            action_context: best_ctx.to_owned(),
            action_score: best_score,
        };

        world.trigger(pick_evt);
    }
}
