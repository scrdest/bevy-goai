use std::borrow::Borrow;
use std::collections::HashMap;
use bevy::prelude::*;
use bevy::ecs::system::{SystemState};
use crate::context_fetchers::{ContextFetcherRequest, ContextFetchResponse};
use crate::considerations::{BatchedConsiderationRequest, ConsiderationMappedToSystemIds, ConsiderationKeyToSystemIdMap};
use crate::curves::{SupportedUtilityCurve, UtilityCurve, UtilityCurveRegistry, resolve_curve_from_name};
use crate::errors::NoCurveMatchStrategyConfig;
use crate::events::AiDecisionRequested;
use crate::lods::{AiLevelOfDetail};
use crate::smart_object::ActionSetStore;
use crate::types::{self, ActionContextRef, ActionScore, ActionTemplateRef};


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
    lod_query: Query<Option<&AiLevelOfDetail>>, 
    mut request_writer: MessageWriter<ContextFetcherRequest>,
) {
    let entity = event.event_target();
    let lod_level = lod_query.get(entity).ok().flatten().map(|lod| lod.get_current_lod());

    let is_disabled = lod_level.map(|lod| lod.is_inactive() ).unwrap_or(false);
    if is_disabled {
        // Early termination - this AI is disabled; generally we'd hope AiDecisionRequested would not even
        // fire in the first place, but weird things can sometimes happen in sufficiently big projects...
        bevy::log::debug!("ai_action_gather_phase: AI {:?} disabled by LOD - ignoring decision request.", entity);
        return;
    }
    
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
            acts.actions.iter()
            .cloned()
            .map(|act| std::sync::Arc::new(act))
            .collect::<Vec<types::ActionTemplateRef>>()
        });

        bevy::log::debug!("ai_action_gather_phase: Available actions for {:?} are: {:#?}", entity, smartobjects.actionset_refs);

        // 2. Emit a request for Context for each ActionTemplate.
        for action_tmpl in available_actions {
            let lod_active = action_tmpl.is_within_lod_range(&lod_level);

            if !lod_active {
                bevy::log::debug!("ai_action_gather_phase: AI {:?}: Skipping template {:?}", entity, action_tmpl.name);
            }

            bevy::log::debug!("ai_action_gather_phase: AI {:?}: Requesting Contexts for template {:?}", entity, action_tmpl.name);
            
            request_writer.write(
                ContextFetcherRequest::new(
                    entity.into(),
                    action_tmpl.clone(),
                )
            );
        }
    }
}


pub fn ai_action_prescoring_phase(
    system_id_map: Res<ConsiderationKeyToSystemIdMap>,
    entity_checker: Query<(Entity, Option<&AiLevelOfDetail>)>, 
    mut reader: MessageReader<ContextFetchResponse>,
    mut request_writer: MessageWriter<BatchedConsiderationRequest>,
) {
    bevy::log::debug_once!("ai_action_prescoring_phase: Triggered ai_action_prescoring_phase()");

    for (msg, msg_id) in reader.read_with_id() {
        bevy::log::debug!("ai_action_prescoring_phase: Processing CF message {:?} (ID: {:?})", msg, msg_id);

        let audience = &msg.audience;
        let exists_check = entity_checker.get(audience.entity());

        if exists_check.is_err() {
            // Sanity check - if an AI is despawned in the meantime, we don't need to process it.
            bevy::log::debug!("ai_action_prescoring_phase: skipping message {:?} - audience AI {:?} no longer exists", msg_id, audience);
            continue;
        }

        let lod_level = exists_check
            .map(|l| l.1)
            .ok()
            .flatten()
            .map(|c| c.get_current_lod())
        ;

        let action_template = &msg.action_template;

        if !action_template.is_within_lod_range(&lod_level) {
            bevy::log::debug!(
                "ai_action_prescoring_phase: skipping message {:?} - audience AI {:?}'s current LOD is out of range for Template {:?}", 
                msg_id, audience, action_template.name
            );
            continue;
        }

        let considerations = &action_template.considerations;
        let contexts = &msg.contexts;

        for ctx in contexts.iter() {
            bevy::log::debug!("ai_action_prescoring_phase: Scoring context for template {:?}: {:#?}", action_template.name, ctx);

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
                entity: audience.clone(),
                scored_action_template: action_template.to_owned(),
                scored_context: ctx.to_owned(),
                considerations: system_ids.collect(),
            };

            request_writer.write(request_batch);
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

    bevy::log::debug!(
        "Adjusted raw score {:?} w/ {:?} (float: {:?}) Considerations to {:?}",
        score,
        num_considerations,
        floaty_num_considerations,
        adjusted_score,
    );

    adjusted_score
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
            ActionTemplateRef, 
            ActionContextRef,
        )
    >::new();

    // Best score reached for this ActionTemplate
    // This is a bit more 'local' than the per-AI score
    let mut best_scoring_template = HashMap::<
        (Entity, ActionTemplateRef), 
        ActionScore
    >::new();

    for msg in messages {
        bevy::log::debug!("AI {:?} - ai_action_scoring_phase: processing Ctx {:?} for Action {:?}", 
            &msg.entity,
            &msg.scored_context, 
            &msg.scored_action_template.name,
        );
        let ai = &msg.entity;

        let curr_best_for_ai = best_scoring_for_ai
            .get(&ai)
            .map(|tup| tup.0)
        ;

        // We do not unwrap curr_best_for_ai fully to be clearer when it's null vs zero.
        if let Some(some_curr_best) = curr_best_for_ai {
            if some_curr_best >= msg.scored_action_template.priority {
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
        }
        let curr_template = msg.scored_action_template.clone();
        
        let best_score_for_template = best_scoring_template
            .get(&(ai.entity(), curr_template.clone()))
        ;

        // The current total score for this AI + Action
        let mut curr_score = types::MAX_CONSIDERATION_SCORE;
        let mut consideration_count: usize = 0;

        for (cons_cnt, cons) in msg.considerations.iter().enumerate() {
            // We'll use the Registry resource if we have one and fall back to the hardcoded pool if we do not.
            let mut maybe_resolved_curve: Option<SupportedUtilityCurve> = match world.get_resource::<UtilityCurveRegistry>() {
                Some(curve_mapping) => curve_mapping.get_curve_by_name(&cons.curve_name),
                None => resolve_curve_from_name(&cons.curve_name),
            };

            if maybe_resolved_curve.is_none() {
                let default_strategy = NoCurveMatchStrategyConfig::default();

                let curve_miss_strategy = world
                    .get_resource::<crate::errors::NoCurveMatchStrategyConfig>()
                    .unwrap_or(&default_strategy)
                ;

                match &curve_miss_strategy.0 {
                    crate::errors::NoCurveMatchStrategy::Panic => {
                        bevy::log::warn!(
                            "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, panicking!", 
                            &msg.entity,
                            &cons.curve_name
                        );
                        panic!("Failed to resolve Curve key to a SupportedUtilityCurve!");
                    },

                    crate::errors::NoCurveMatchStrategy::SkipConsiderationWithLog => {
                        bevy::log::warn!(
                            "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, skipping Consideration {:?}!", 
                            &msg.entity,
                            &cons.curve_name,
                            &cons.func_name,
                        );
                        continue;
                    },

                    crate::errors::NoCurveMatchStrategy::SkipActionWithLog => {
                        bevy::log::warn!(
                            "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, skipping ActionTemplate {:?}!", 
                            &msg.entity,
                            &cons.curve_name,
                            &curr_template.name,
                        );
                        break;
                    },

                    crate::errors::NoCurveMatchStrategy::DefaultCurveWithLog(curve_resolver) => {
                        let resolved = curve_resolver(cons.curve_name.borrow());
                        
                        bevy::log::warn!(
                            "AI {:?} - Curve key {:?} resolved using fallback Curve {:?}", 
                            &msg.entity,
                            &cons.curve_name,
                            &resolved,
                        );

                        maybe_resolved_curve = Some(resolved)
                    },

                    crate::errors::NoCurveMatchStrategy::DefaultCurveWithoutLog(curve_resolver) => {
                        let resolved = curve_resolver(cons.curve_name.borrow());
                        maybe_resolved_curve = Some(resolved)
                    },
                }
            }

            // We can safely unwrap this as any handling/panicking has been done earlier.
            let resolved_curve = maybe_resolved_curve.unwrap();

            match cons.consideration_systemid {
                Err(_) => bevy::log::debug!(
                    "AI {:?} - Failed to resolve Consideration '{:}' to a System!", 
                    &msg.entity,
                    &cons.func_name
                ),
                Ok(system_id) => {
                    let res = world.run_system_with(
                        system_id,
                        (
                            msg.entity.entity(),
                            msg.entity.entity(),
                            msg.scored_context.to_owned(),
                        )
                    );

                    if res.is_err() {
                        bevy::log::debug!(
                            "AI {:?} - Consideration '{:}' errored: {:?}", 
                            &msg.entity, 
                            &cons.func_name, 
                            &res
                        );
                        continue;
                    };

                    let raw_score = res.expect(
                        "Failed to unwrap a res to a raw_score. It should always be Ok, but is Err somehow."
                    );

                    let (true_min, true_max) = match cons.min <= cons.max {
                        true => (cons.min, cons.max),
                        false => {
                            bevy::log::error!(
                                "Min/Max values for Consideration {:?} in Action {:?} 
                                were flipped, min={:?} > max={:?}. 
                                They have been flipped back so Min<=Max for you for now. 
                                This fixup is not guaranteed to be in place in future versions of the library!",
                                cons.func_name,
                                curr_template.name,
                                cons.min,
                                cons.max,
                            );
                            (cons.max, cons.min)
                        }
                    };

                    // Remap the raw Consideration score (arbitrary value) to a unit interval. 
                    // Values outside of range get saturated to min/max (as appropriate), so 
                    // e.g. if min = -1 and raw_score = -5, we read the raw_score as just -1.
                    // Similarly if max = -4 and raw_score = -1, we read the raw_score as just -4.
                    let rescaled_score = (raw_score - true_min).clamp(true_min, true_max) / (true_max - true_min);

                    let curr_template_best = best_score_for_template.copied().unwrap_or(
                        types::MIN_CONSIDERATION_SCORE
                    );

                    let score = resolved_curve.sample_safe(rescaled_score);

                    // The actual (raw) score is the product of all Consideration scores so far.
                    curr_score *= score;

                    bevy::log::debug!(
    "AI {:?} - Consideration '{:}' for Action {:?}: 
    - Raw score => {:?}
    - Rescaled w/ min/max => {:?}
    - Adjusted w/ Curve {:?} => {:?}
    - Current running total score for Action => {:?}",
                        msg.entity,
                        cons.func_name,
                        curr_template.name,
                        raw_score,
                        rescaled_score,
                        cons.curve_name,
                        score,
                        curr_score,
                    );

                    // There is a superior Context for this ActionTemplate.
                    // We don't need to bother checking other Considerations for this Context, 
                    // as it will not get picked anyway.
                    if curr_template_best >= curr_score {
                        
                        bevy::log::debug!(
                            "AI {:?} - Consideration '{:}' for Action {:?} - score {:?} is below the template best of {:?}, discarding the Context.",
                            msg.entity,
                            cons.func_name,
                            curr_template.name,
                            score,
                            curr_template_best,
                        );
                        break;
                    }

                    // We need to know how many Considerations we have processed for later.
                    // Enumerate starts at zero, so we need to add one to adjust.
                    consideration_count = cons_cnt + 1;
                }
            }
        }
        
        best_scoring_template.insert(
            (ai.entity(), curr_template.clone()), 
            // Each Context has the same amount of Considerations and same Priority, 
            // so we can store and compare raw scores without the other cruft.
            curr_score
        );

        let adjusted_score = consideration_adjustment(
            curr_score, 
            consideration_count,
        );

        // todo: add a parametrizeable amount of randomness for break-evens
        let prioritized_score = adjusted_score * curr_template.priority;

        match prioritized_score > curr_best_for_ai.unwrap_or(types::MIN_CONSIDERATION_SCORE) {
            false => {
                bevy::log::debug!(
                    "AI {:?} - Score for Action {:?} = {:?} is below the current best of {:?}. Ignoring.",
                    &msg.entity,
                    curr_template.name,
                    prioritized_score,
                    curr_best_for_ai,
                );
            },
            true => {
                bevy::log::debug!(
                    "AI {:?} - Score for Action {:?} = {:?} beats the current best of {:?}. Promoting to new best.",
                    &msg.entity,
                    curr_template.name,
                    prioritized_score,
                    curr_best_for_ai,
                );

                // Update frontrunners for each AI processed.
                best_scoring_for_ai.insert(
                    ai.entity(), 
                    (prioritized_score, curr_template, msg.scored_context)
                );
            }
        }
    }

    for (
        ai, (
            best_score, 
            best_template, 
            best_ctx
        )
    ) in best_scoring_for_ai {
        bevy::log::info!(
            "Picking Action {:?} w/ Score {:?} for AI {:?}...", 
            &best_template.name,
            &best_score,
            &ai,
        );

        let pick_evt = crate::events::AiActionPicked {
            entity: ai.entity(),
            action_key: best_template.action_key.to_owned(),
            action_name: best_template.name.to_owned(),
            action_context: best_ctx.to_owned(),
            action_score: best_score,
        };

        world.trigger(pick_evt);
    }
}
