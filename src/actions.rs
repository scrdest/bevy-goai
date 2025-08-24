// Actions

use std::borrow::Borrow;
use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{List, Reflect, func::FunctionRegistry, func::DynamicFunction, func::ArgList};
use serde::{Serialize, Deserialize};
use crate::arg_values::ContextValue;
use crate::errors::{DynResolutionError};
use crate::events::ActionEvent;
use crate::type_registry::{IsTypeRegistryIdentifier};
use crate::utility_concepts::{ContextFetcherIdentifier, CurveIdentifier, ConsiderationIdentifier};


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct DynFuncName(String);

impl From<String> for DynFuncName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Borrow<str> for DynFuncName {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Borrow<str> for &DynFuncName {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl IsTypeRegistryIdentifier for DynFuncName {}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
pub struct ConsiderationData {
    #[serde(rename="consideration")]
    func_name: ConsiderationIdentifier,

    #[serde(rename="curve")]
    curve_name: CurveIdentifier,

    min: f32,
    max: f32,
}

struct RunnableConsideration<'a, 'b: 'a> {
    func: DynamicFunction<'a>,
    curve: DynamicFunction<'b>,
    min: f32,
    max: f32,
}

pub type ActionContext = HashMap<String, ContextValue>;



#[derive(Clone, Reflect, Debug)]
pub(crate) struct Action {
    /// A GOAI action is effectively an ActionTemplate + a selected Context. 
    /// 
    // pub(crate) func: TypeRegistryFuncIdentifier,
    pub(crate) name: String,
    pub(crate) context: ActionContext,
    pub(crate) event_id: crate::events::GoaiActionEventId,
}

#[derive(Clone, Reflect, Debug)]
pub(crate) struct ScoredAction {
    /// 
    pub(crate) action: Action,
    pub(crate) score: f32,
}


#[derive(Reflect, Serialize, Deserialize, Debug, Clone)]
pub struct ActionTemplate {
    /// An ActionTemplate is a 'partial' Action (in the sense of a partial function).
    /// It represents an abstract activity an AI may undertake without a specific target.
    /// 
    /// For example, OpenDoor is an ActionTemplate - it becomes an Action when we specify WHICH door to open,
    /// which we'll refer to by the quasi-generic-esque syntax OpenDoor<SomeDoor>. 
    /// The square-bracketed value(s) are what GOAI (following IAUS) calls a Context.
    /// 
    /// In total, an ActionTemplate is: 
    /// 1) a ref to a function we will run as the Action (e.g. open_door_handler()).
    /// 2) a ref to a function that returns possible Contexts (e.g. get_adjacent_doors()).
    /// 3) a sequence of refs to functions that will score all of these (e.g. [distance_to_pawn()]).
    /// 4) a multiplier for the final score to make certain activities intrinsically higher priority (e.g. 1.5).
    // 
    // name = identifier. Two ActionTemplates may share the same function (as an implementation detail), 
    //                    but represent very different logical activities. This helps AI designers not go mad.
    pub(crate) name: String, 
    #[serde(rename="context_fetcher")]
    pub(crate) context_fetcher_name: ContextFetcherIdentifier,
    pub(crate) considerations: Vec<ConsiderationData>,
    pub(crate) priority: f32,
    pub(crate) event: crate::events::GoaiActionEventId,
}

impl ActionTemplate  {
    fn try_resolve_action_function(&self, fn_name: &str, registry: &FunctionRegistry) -> Result<DynamicFunction, DynResolutionError> {
        let func = registry.get(&fn_name);
        func
            .ok_or(DynResolutionError::NotInRegistry(fn_name.to_owned()))
            .map(|fun| fun.to_owned())
    }

    fn resolve_action_function(&self, fn_name: &str, registry: &FunctionRegistry) -> DynamicFunction {
        self.try_resolve_action_function(fn_name, registry).expect("Failed to resolve dynamic function from a name!")
    }

    fn resolve_context_fetcher(&self, registry: &FunctionRegistry) -> DynamicFunction {
        self.try_resolve_action_function(&self.context_fetcher_name.borrow(), registry).expect("Failed to resolve dynamic ContextFetcher function from a name!")
    }

    fn resolve_consideration(&self, consideration_name: &str, registry: &FunctionRegistry) -> DynamicFunction {
        self.try_resolve_action_function(consideration_name, registry).expect("Failed to resolve dynamic Consideration function from a name!")
    }

    fn resolve_curve(&self, curve_name: &str, registry: &FunctionRegistry) -> DynamicFunction {
        self.try_resolve_action_function(curve_name, registry).expect("Failed to resolve dynamic Curve function from a name!")
    }

    fn get_contexts(&self, registry: &FunctionRegistry) -> Vec<ActionContext> {
        // todo: error-handling
        // todo: put world n' stuff into arglist
        let context_fetcher = self.resolve_context_fetcher(registry);
        let args = ArgList::new();
        let raw_result = context_fetcher.call(args);
        let result = raw_result.unwrap().unwrap_owned();
        let output: Vec<ActionContext> = result.try_take().unwrap();
        output
    }

    pub(crate) fn run_considerations(&self, registry: &FunctionRegistry, best_score_cutoff: Option<f32>) -> Option<ScoredAction> {
        let callable_considerations: Vec<RunnableConsideration> = self.considerations.iter().map(
            |dynamiccons| {
                let consdata: &ConsiderationData = dynamiccons.try_downcast_ref().unwrap();
                let func = self.resolve_consideration(&consdata.func_name.borrow(), registry);
                let curve = self.resolve_curve(&consdata.curve_name.borrow(), registry);
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

        let mut best_ctx: Option<&dyn PartialReflect> = None;
        let mut best_score: f32 = best_score_cutoff.unwrap_or(0.);

        let contexts = self.get_contexts(registry);
        
        for context in contexts.iter() {
            bevy::log::debug!("Scoring context for template {:?}: {:#?}", self.name, context);
            let mut curr_score: f32 = 1.;
            let mut ignored: bool = false;

            for consideration in callable_considerations.iter() {
                let args = ArgList::new()
                    .with_ref(context)
                ;
                let dyn_score = consideration.func.call(args).unwrap().unwrap_owned();
                let cast_score = dyn_score.try_take();

                let score: f32 = cast_score.unwrap_or(0.);
                curr_score *= score;

                if curr_score <= best_score {
                    // early termination; it's not gonna be worth it
                    ignored = true;
                    break;
                };
            }
            
            bevy::log::debug!("Scored context for template {:?}: {:#?} => score={:?}, best={:?} ignored={:?}", self.name, context, curr_score, best_score, ignored);
            
            if ignored { 
                // break inner loop, skip the whole context - it's no bueno
                continue 
            };

            if best_ctx.is_none() || (curr_score > best_score) {
                best_score = curr_score;
                best_ctx = Some(context);
            };
        };

        bevy::log::debug!("Best context for template {:?}: {:#?}", self.name, best_ctx);

        match best_ctx {
            None => None,
            Some(ctx) => {
                let context: ActionContext =  ctx.try_downcast_ref::<ActionContext>().unwrap().to_owned();
                let name = self.name.to_owned();
                let action = Action {
                    name, 
                    context,
                    event_id: self.event.clone(),
                };
                Some(ScoredAction {
                    action,
                    score: best_score,
                })
            }
        }
    }
}
