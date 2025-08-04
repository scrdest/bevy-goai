// Actions

use std::borrow::Borrow;

use bevy::prelude::*;
use bevy::reflect::func::ArgList;
use bevy::reflect::{List};
use bevy::reflect::{Reflect, DynamicList, DynamicMap, func::FunctionRegistry, func::DynamicFunction};
use serde::{Serialize, Deserialize};
use crate::errors::{DynResolutionError};
use crate::type_registry::{TypeRegistryIdentifier, TypeRegistryIdentifierFor};
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
impl TypeRegistryIdentifier for DynFuncName {}


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

pub(crate) struct Action {
    /// A GOAI action is effectively an ActionTemplate + a selected Context. 
    /// 
    name: String,
    func: DynFuncName,
    context: DynamicMap,
}

pub(crate) struct ScoredAction {
    /// 
    action: Action,
    score: f32,
}


#[derive(Reflect, Serialize, Deserialize, Debug)]
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
    name: String, 
    pub(crate) function: DynFuncName,
    #[serde(rename="context_fetcher")]
    pub(crate) context_fetcher_name: ContextFetcherIdentifier,
    considerations: Vec<ConsiderationData>,
    priority: f32,
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

    fn get_contexts(&self, registry: &FunctionRegistry) -> DynamicList {
        // todo: error-handling
        // todo: put world n' stuff into arglist
        let context_fetcher = self.resolve_context_fetcher(registry);
        let args = ArgList::new();
        let result = context_fetcher.call(args).unwrap().unwrap_owned();
        let output: DynamicList = result.try_take().unwrap();
        output
    }

    fn run_considerations(&self, registry: &FunctionRegistry) -> Option<ScoredAction> {
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
        let mut best_score: f32 = 0.;

        let contexts = self.get_contexts(registry);
        
        for context in contexts.iter() {
            let mut curr_score: f32 = 1.;
            let mut ignored: bool = false;

            for consideration in callable_considerations.iter() {
                let args = ArgList::new().with_ref(context);
                let dyn_score = consideration.func.call(args).unwrap().unwrap_owned();
                let cast_score = dyn_score.try_take();

                let score: f32 = cast_score.unwrap_or(0.);
                curr_score *= score;

                if curr_score <= best_score {
                    ignored = true;
                    break;
                };
            
            if ignored { 
                // break inner loop, skip the whole context - it's no bueno
                continue 
            };

            if best_ctx.is_none() || curr_score > best_score {
                best_score = curr_score;
                best_ctx = Some(context);
            }};
        };

        match best_ctx {
            None => None,
            Some(ctx) => {
                let context: DynamicMap =  ctx.to_dynamic().try_take().unwrap();
                let name = self.name.to_owned();
                let func = self.function.to_owned();
                let action = Action {
                    name, 
                    func, 
                    context,
                };
                Some(ScoredAction {
                    action,
                    score: best_score,
                })
            }
        }
    }
}
