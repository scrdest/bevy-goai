// Actions

use std::any::TypeId;
use std::borrow::Borrow;
use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{List, FromType, Reflect, std_traits::ReflectDefault, TypeRegistration, TypeData, func::FunctionRegistry, func::DynamicFunction, func::ArgList};
use serde::{Serialize, Deserialize};
use crate::action_runtime::ActionState;
use crate::arg_values::ContextValue;
use crate::errors::{DynResolutionError};
use crate::type_registry::{IsTypeRegistryIdentifier, TypeRegistryTypeIdentifier};
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


#[derive(Event)]
pub enum GoaiActionEvent {
    Zero(ActionState, Option<ActionContext>),
    One(ActionState, Option<ActionContext>),
    Two(ActionState, Option<ActionContext>),
    Three(ActionState, Option<ActionContext>),
    Four(ActionState, Option<ActionContext>),
    Five(ActionState, Option<ActionContext>),
    Six(ActionState, Option<ActionContext>),
    Seven(ActionState, Option<ActionContext>),
    Eight(ActionState, Option<ActionContext>),
    Nine(ActionState, Option<ActionContext>),
    Ten(ActionState, Option<ActionContext>),
    Eleven(ActionState, Option<ActionContext>),
    Twelve(ActionState, Option<ActionContext>),
    Thirteen(ActionState, Option<ActionContext>),
    Fourteen(ActionState, Option<ActionContext>),
    Fifteen(ActionState, Option<ActionContext>),
    Sixteen(ActionState, Option<ActionContext>),
}

type GoaiActionEventId = u32;

impl GoaiActionEvent {
    pub(crate) fn from_id_and_context(id: GoaiActionEventId, context: Option<ActionContext>) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Zero(ActionState::Running, context)),
            1 => Ok(Self::One(ActionState::Running, context)),
            2 => Ok(Self::Two(ActionState::Running, context)),
            3 => Ok(Self::Three(ActionState::Running, context)),
            4 => Ok(Self::Four(ActionState::Running, context)),
            5 => Ok(Self::Five(ActionState::Running, context)),
            6 => Ok(Self::Six(ActionState::Running, context)),
            7 => Ok(Self::Seven(ActionState::Running, context)),
            8 => Ok(Self::Eight(ActionState::Running, context)),
            9 => Ok(Self::Zero(ActionState::Running, context)),
            10 => Ok(Self::Ten(ActionState::Running, context)),
            11 => Ok(Self::Eleven(ActionState::Running, context)),
            12 => Ok(Self::Twelve(ActionState::Running, context)),
            13 => Ok(Self::Thirteen(ActionState::Running, context)),
            14 => Ok(Self::Fourteen(ActionState::Running, context)),
            15 => Ok(Self::Fifteen(ActionState::Running, context)),
            16 => Ok(Self::Sixteen(ActionState::Running, context)),
            _ => Err(format!("Id {} outside of supported range (max: 16)", id))
        }
    }

    pub(crate) fn get_state(&self) -> &ActionState {
        match self {
            Self::Zero(state, _) => state,
            Self::One(state, _) => state,
            Self::Two(state, _) => state,
            Self::Three(state, _) => state,
            Self::Four(state, _) => state,
            Self::Five(state, _) => state,
            Self::Six(state, _) => state,
            Self::Seven(state, _) => state,
            Self::Eight(state, _) => state,
            Self::Nine(state, _) => state,
            Self::Ten(state, _) => state,
            Self::Eleven(state, _) => state,
            Self::Twelve(state, _) => state,
            Self::Thirteen(state, _) => state,
            Self::Fourteen(state, _) => state,
            Self::Fifteen(state, _) => state,
            Self::Sixteen(state, _) => state,
        }
    }

    pub(crate) fn get_context(&self) -> &Option<ActionContext> {
        match self {
            Self::Zero(_, ctx) => ctx,
            Self::One(_, ctx) => ctx,
            Self::Two(_, ctx) => ctx,
            Self::Three(_, ctx) => ctx,
            Self::Four(_, ctx) => ctx,
            Self::Five(_, ctx) => ctx,
            Self::Six(_, ctx) => ctx,
            Self::Seven(_, ctx) => ctx,
            Self::Eight(_, ctx) => ctx,
            Self::Nine(_, ctx) => ctx,
            Self::Ten(_, ctx) => ctx,
            Self::Eleven(_, ctx) => ctx,
            Self::Twelve(_, ctx) => ctx,
            Self::Thirteen(_, ctx) => ctx,
            Self::Fourteen(_, ctx) => ctx,
            Self::Fifteen(_, ctx) => ctx,
            Self::Sixteen(_, ctx) => ctx,
        }
    }
}


#[derive(Clone)]
pub struct ReflectIntoEvent {
    builder: fn(GoaiActionEventId, ActionContext) -> GoaiActionEvent
}

impl ReflectIntoEvent {
    pub fn builder(&self, id: GoaiActionEventId, context: ActionContext) -> GoaiActionEvent {
        (self.builder)(id, context)
    }
}

impl<E: Reflect + ActionEvent + Default> FromType<E> for ReflectIntoEvent {
    fn from_type() -> Self {
        Self {
            builder: |id, ctx| GoaiActionEvent::from_id_and_context(id, Some(ctx)).unwrap(),
        }
    }
}

pub trait ActionEventFactory: Reflect {
    type AsEvent: ActionEvent;

    fn to_action_event(&self) -> Self::AsEvent;
}

pub trait ActionEvent: Reflect {
    type AsEvent: Event + Reflect;

    fn from_context(context: ActionContext) -> Self::AsEvent;

    fn from_context_reflect(context: ActionContext) -> Box<Self::AsEvent> {
        let base = Self::from_context(context);
        Box::new(base)
    }
}

/// Marker trait for ActionEvents that do not make use of the Context, 
/// and can therefore be implemented cheaply using the type's Default implementation. 
/// This is meant for either: 
/// 
/// (1) events that store no data at all (i.e. empty structs deriving Event), or 
/// (2) events that only store stuff in safely defaultable containers to be filled in later.
pub trait ContextFreeActionEvent: ActionEvent + Default {
    fn from_context(_context: ActionContext) -> Self {
        Self::default()
    }
}


#[derive(Clone, Reflect, Debug)]
pub(crate) struct Action {
    /// A GOAI action is effectively an ActionTemplate + a selected Context. 
    /// 
    // pub(crate) func: TypeRegistryFuncIdentifier,
    pub(crate) name: String,
    pub(crate) context: ActionContext,
    pub(crate) event_id: GoaiActionEventId,
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
    pub(crate) event: GoaiActionEventId,
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
