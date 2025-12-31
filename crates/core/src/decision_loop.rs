use std::borrow::Borrow;
use std::collections::HashMap;
use bevy::prelude::*;
use crate::ai::{AIController};
use crate::context_fetchers::{ContextFetcherKeyToSystemMap};
use crate::considerations::{ConsiderationKeyToSystemMap};
use crate::curves::{SupportedUtilityCurve, UtilityCurve, UtilityCurveRegistry, resolve_curve_from_name};
use crate::errors::NoCurveMatchStrategyConfig;
use crate::events::AiDecisionRequested;
use crate::lods::{AiLevelOfDetail};
use crate::smart_object::ActionSetStore;
use crate::types::{self, ActionContextRef, ActionScore, ActionTemplateRef};


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
        "Adjusted raw score {:?} w/ {:?} Considerations (float: {:?}) to {:?}",
        score,
        num_considerations,
        floaty_num_considerations,
        adjusted_score,
    );

    adjusted_score
}


/// Core AI decision loop. 
/// 
/// Finds the `Action` with the highest Utility Score and triggers an `ActionPickedEvent`.
/// 
/// Note that this is an Observer that runs for an *individual* `AiController`. 
/// Running all AIs at all times by default would be a waste of compute.
/// 
/// We iterate through all available ActionTemplates (from all available sources), 
/// then - after filtering out obvious non-starters - fetch potential `Contexts` for 
/// them to form a full-fledged candidate `Action`. 
/// 
/// We then apply Considerations specified in the ActionTemplate for each candidate 
/// in sequence, each returning a raw score, which is then adjusted using UtilityCurves 
/// to calculate the true Axis Score.
/// 
/// The final score for an Action is a product of all Axis Scores starting from 1.0 
/// (a classic simple fold/reduce type function), strictly non-increasing. There is 
/// also a nonlinear 'bonus' applied rewarding Actions with more Considerations to 
/// adjust for the downward drift adding extra Considerations incurs.
/// 
/// This is a gauntlet; any candidate whose value drops to zero is eliminated instantly, 
/// as is any candidate whose score dips below the frontrunner. Considerations may be 
/// expensive (multiple raycasts, complex formulas, etc.) so we avoid paying for those 
/// we are never going to actually use.
/// 
/// ContextFetchers, Considerations, and Curves used by this system can all be provided 
/// by the user or by third-party plugins! 
/// 
/// We refer to them by String keys in SmartObject definitions and resolve them at runtime 
/// using a couple of custom Resources provided by Cortex; see `app.register_consideration()`, 
/// `app.register_context_fetcher()` and `app.register_utility_curve()` for API details.
pub fn decision_engine(
    event: On<AiDecisionRequested>,
    world_ref: &World, 
    actionset_store: Res<ActionSetStore>,
    context_fetcher_system_map: Res<ContextFetcherKeyToSystemMap>,
    consideration_system_map: Res<ConsiderationKeyToSystemMap>,
    entity_checker: Query<Entity, With<AIController>>, 
    lod_query: Query<Option<&AiLevelOfDetail>>, 
    utility_curve_registry: Option<Res<UtilityCurveRegistry>>,
    no_match_strategy_config: Option<Res<NoCurveMatchStrategyConfig>>,
    mut commands: Commands,
) {
    let audience = event.event_target();

    let exist_check = entity_checker.get(audience);
    if exist_check.is_err() {
        // Early termination - the AI the decision was requested for either got despawned or the request 
        // was malformed and was pointed at something that was not an AI in the first place.
        bevy::log::debug!("decision_engine: Decision request target {:?} is not an AI - ignoring the request.", audience);
        return;
    }
    
    let lod_level = lod_query
        .get(audience)
        .ok()
        .flatten()
        .map(|lod| lod.get_current_lod())
    ;

    let is_disabled = lod_level.map(|lod| lod.is_inactive() ).unwrap_or(false);
    if is_disabled {
        // Early termination - this AI is disabled; generally we'd hope AiDecisionRequested would not even
        // fire in the first place, but weird things can sometimes happen in sufficiently big projects...
        bevy::log::debug!("decision_engine: AI {:?} disabled by LOD - ignoring decision request.", audience);
        return;
    }
    
    // Best score reached for this AI, globally
    // If any batch Score dips below this, we can discard the whole batch immediately 
    // as it cannot possibly beat the current best.
    let mut best_scoring_triple: Option<(ActionScore, ActionTemplateRef, ActionContextRef)> = None;

    // Best score reached for this ActionTemplate
    // This is a bit more 'local' than the per-AI score
    let mut best_scoring_template = HashMap::<
        (Entity, ActionTemplateRef), 
        ActionScore
    >::new();
    
    let maybe_smartobjects = &event.smart_objects;
    
    // 1. Gather ActionSets from Smart Objects
    let smartobjects = match maybe_smartobjects {
        None => {
            // Early return - we have no real options in this case.
            // Note that there is no notion of available Actions *NOT* tied to a SO; at
            // minimum, you'd have a SO with the key representing *the Controller itself*.
            bevy::log::debug!("decision_engine: AI {:?} - no SmartObjects available, idling", audience);
            return;
        }
        Some(sos) => sos
    };

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
        // .collect::<Vec<types::ActionTemplateRef>>()
    });

    bevy::log::debug!(
        "decision_engine: AI {:?} - available Actions are: {:#?}", 
        audience, &smartobjects.actionset_refs
    );

    // 2. Emit a request for Context for each ActionTemplate.
    for action_template in available_actions {
        if !action_template.is_within_lod_range(&lod_level) {
            bevy::log::debug!(
                "decision_engine: AI {:?} - skipping Template {:?} - current LOD {:?} does not allow for processing.", 
                &action_template.name, &audience, &lod_level,
            );
            continue;
        }

        bevy::log::debug!("decision_engine: AI {:?} - requesting Contexts for Template {:?}", &audience, &action_template.name);
        
        // Request Contexts using registered ContextFetcher Systems
        let cf_system = context_fetcher_system_map.mapping
            .get(&action_template.context_fetcher_name.0)
            .cloned()
        ;
        
        let contexts = match cf_system {
            Some(system_guard) => {
                let res = system_guard.write().map(|mut cf_system| {
                    cf_system.run_readonly(
                        (
                            audience,
                            // TODO: FIX TO PAWN!
                            audience,
                        ),
                        world_ref,
                    )
                });

                if res.is_err() {
                    bevy::log::error!(
                        "AI {:?} - ContextFetcher '{:?}' errored - lock poisoned ({:?})!", 
                        &audience, 
                        &action_template.context_fetcher_name, 
                        &res,
                    );
                    // If the lock has been poisoned, we've had a panic inside it, 
                    // so we're in uncharted waters - abort before things get worse.
                    panic!("ContextFetcher failed - lock poisoned!");
                };

                let res = res.unwrap();

                if res.is_err() {
                    bevy::log::error!(
                        "AI {:?} - ContextFetcher '{:?}' errored: {:?}", 
                        &audience, 
                        &action_template.context_fetcher_name, 
                        &res,
                    );
                    continue;
                };

                res.expect("ContextFetcher result is Err - this should not be possible!")
            },
            None => {
                bevy::log::error!(
                    "AI {:?} - ContextFetcher key '{:?}' could not be resolved to a System!", 
                    &audience, 
                    &action_template.context_fetcher_name, 
                );
                continue;
            }
        };

        for ctx in contexts {
            let ctx_ref = std::sync::Arc::new(ctx);
            
            bevy::log::debug!("AI {:?} - processing Ctx {:?} for Action {:?}", 
                &audience,
                &ctx_ref, 
                &action_template,
            );
            let ai = &audience;

            let curr_best_for_ai = best_scoring_triple
                .clone()
                .map(|tup| tup.0);

            // We do not unwrap curr_best_for_ai fully to be clearer when it's null vs zero.
            if let Some(some_curr_best) = curr_best_for_ai {
                if some_curr_best >= action_template.priority {
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
            
            let best_score_for_template = best_scoring_template
                .get(&(ai.entity(), action_template.to_owned()))
            ;

            // The current total score for this AI + Action
            let mut curr_score = types::MAX_CONSIDERATION_SCORE;
            let mut consideration_count: usize = 0;

            for (cons_cnt, cons) in action_template.considerations.iter().enumerate() {
                // We'll use the Registry resource if we have one and fall back to the hardcoded pool if we do not.
                let mut maybe_resolved_curve: Option<SupportedUtilityCurve> = utility_curve_registry
                    .as_ref()
                    .map(|curve_mapping| 
                        curve_mapping.get_curve_by_name(&cons.curve_name)
                    )
                    .flatten()
                    .or_else(|| resolve_curve_from_name(&cons.curve_name))
                ;

                if maybe_resolved_curve.is_none() {
                    let curve_miss_strategy = no_match_strategy_config
                        .as_ref()
                        .map(|conf| conf.get_current_value())
                    ;

                    match curve_miss_strategy {
                        None => {
                            // This is a duplicate of the Panic strategy as indicated by the Default impl. 
                            // We COULD create a fallback value earlier, but that would cost us an extra 
                            // `.clone()` that we can kinda do without here just as well.
                            bevy::log::warn!(
                                "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, default behavior - panicking!", 
                                &audience,
                                &cons.curve_name
                            );
                            panic!("Failed to resolve Curve key to a SupportedUtilityCurve!");
                        },

                        Some(crate::errors::NoCurveMatchStrategy::Panic) => {
                            bevy::log::warn!(
                                "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, panicking!", 
                                &audience,
                                &cons.curve_name
                            );
                            panic!("Failed to resolve Curve key to a SupportedUtilityCurve!");
                        },

                        Some(crate::errors::NoCurveMatchStrategy::SkipConsiderationWithLog) => {
                            bevy::log::warn!(
                                "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, skipping Consideration {:?}!", 
                                &audience,
                                &cons.curve_name,
                                &cons.func_name,
                            );
                            continue;
                        },

                        Some(crate::errors::NoCurveMatchStrategy::SkipActionWithLog) => {
                            bevy::log::warn!(
                                "AI {:?} - Failed to resolve Curve key {:?} to a SupportedUtilityCurve, skipping ActionTemplate {:?}!", 
                                &audience,
                                &cons.curve_name,
                                &action_template.name,
                            );
                            break;
                        },

                        Some(crate::errors::NoCurveMatchStrategy::DefaultCurveWithLog(curve_resolver)) => {
                            let resolved = curve_resolver(cons.curve_name.borrow());
                            
                            bevy::log::warn!(
                                "AI {:?} - Curve key {:?} resolved using fallback Curve {:?}", 
                                &audience,
                                &cons.curve_name,
                                &resolved,
                            );

                            maybe_resolved_curve = Some(resolved)
                        },

                        Some(crate::errors::NoCurveMatchStrategy::DefaultCurveWithoutLog(curve_resolver)) => {
                            let resolved = curve_resolver(cons.curve_name.borrow());
                            maybe_resolved_curve = Some(resolved)
                        },
                    }
                }

                // We can safely unwrap this as any handling/panicking has been done earlier.
                let resolved_curve = maybe_resolved_curve.unwrap();

                let consideration_system = consideration_system_map.mapping
                    .get(&cons.func_name)
                ;

                match consideration_system {
                    None => bevy::log::debug!(
                        "AI {:?} - Failed to resolve Consideration '{:}' to a System!", 
                        &audience,
                        &cons.func_name
                    ),

                    Some(system_guard) => {
                        let res = system_guard
                            .write()
                            .map(|mut consideration_system| {
                                consideration_system.run_readonly(
                                (
                                        audience.entity(),
                                        audience.entity(),
                                        ctx_ref.clone(),
                                    ),
                                    world_ref,
                                )
                            })
                        ;

                        if res.is_err() {
                            bevy::log::debug!(
                                "AI {:?} - Consideration '{:}' errored - lock poisoned ({:?})!", 
                                &audience, 
                                &cons.func_name, 
                                &res
                            );
                            panic!("Consideration failed - lock poisoned!");
                        };

                        let res = res.unwrap();

                        if res.is_err() {
                            bevy::log::debug!(
                                "AI {:?} - Consideration '{:}' errored: {:?}", 
                                &audience, 
                                &cons.func_name, 
                                &res
                            );
                            curr_score = types::MIN_CONSIDERATION_SCORE - 1.;
                            break;
                        };

                        let raw_score = res.expect(
                            "Failed to unwrap a Consideration result to a raw_score. 
                            It should always be Ok, but is somehow an Err value."
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
                                    &action_template.name,
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
                            audience,
                            cons.func_name,
                            &action_template.name,
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
                                audience,
                                cons.func_name,
                                &action_template.name,
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
                (ai.entity(), action_template.clone()), 
                // Each Context has the same amount of Considerations and same Priority, 
                // so we can store and compare raw scores without the other cruft.
                curr_score
            );

            let adjusted_score = consideration_adjustment(
                curr_score, 
                consideration_count,
            );

            // todo: add a parametrizeable amount of randomness for break-evens
            let prioritized_score = adjusted_score * action_template.priority;

            match prioritized_score > curr_best_for_ai.unwrap_or(types::MIN_CONSIDERATION_SCORE) {
                false => {
                    bevy::log::debug!(
                        "AI {:?} - Score for Action {:?} = {:?} is below the current best of {:?}. Ignoring.",
                        &audience,
                        &action_template.name,
                        prioritized_score,
                        curr_best_for_ai,
                    );
                },
                true => {
                    bevy::log::debug!(
                        "AI {:?} - Score for Action {:?} = {:?} beats the current best of {:?}. Promoting to new best.",
                        &audience,
                        &action_template.name,
                        prioritized_score,
                        curr_best_for_ai,
                    );

                    // Update frontrunner.
                    best_scoring_triple = Some((prioritized_score, action_template.clone(), ctx_ref))
                }
            }
        }
    }

    match best_scoring_triple {
        None => {
            bevy::log::debug!("")
        }
        Some(best_tuple) => {
            let (
                best_score, 
                best_template, 
                best_context
            ) = best_tuple;

            bevy::log::info!(
                "Picking Action {:?} w/ Score {:?} for AI {:?}...", 
                &best_template.name,
                &best_score,
                &audience,
            );

            let pick_evt = crate::events::AiActionPicked {
                entity: audience.entity(),
                action_key: best_template.action_key.to_owned(),
                action_name: best_template.name.to_owned(),
                action_context: best_context.to_owned(),
                action_score: best_score,
            };

            commands.trigger(pick_evt);
        }
    }
}

