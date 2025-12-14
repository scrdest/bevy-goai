//! Actions
use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{Reflect};
use serde::{Serialize, Deserialize};

use crate::arg_values::ContextValue;
use crate::types;
use crate::utility_concepts::{ContextFetcherIdentifier, CurveIdentifier, ConsiderationIdentifier};


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
pub struct ConsiderationData {
    #[serde(rename="consideration")]
    pub(crate) func_name: ConsiderationIdentifier,

    #[serde(rename="curve")]
    pub(crate) curve_name: CurveIdentifier,

    pub(crate) min: types::ActionScore,
    pub(crate) max: types::ActionScore,
}

impl ConsiderationData {
    pub fn new(
        func_name: ConsiderationIdentifier,
        curve_name: CurveIdentifier,
        min: types::ActionScore,
        max: types::ActionScore,
    ) -> Self {
        Self {
            func_name,
            curve_name,
            min, 
            max, 
        }
    }
}

pub type ActionContext = HashMap<String, ContextValue>;

#[derive(Clone, Reflect, Debug)]
pub struct Action {
    /// A GOAI action is effectively an ActionTemplate + a selected Context. 
    /// 
    // pub(crate) func: TypeRegistryFuncIdentifier,
    pub name: String,
    pub context: ActionContext,
    pub action_key: String,
}

#[derive(Clone, Reflect, Debug)]
pub struct ScoredAction {
    /// 
    pub action: Action,
    pub score: types::ActionScore,
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
    pub name: String, 
    #[serde(rename="context_fetcher")]
    pub context_fetcher_name: ContextFetcherIdentifier,
    pub considerations: Vec<ConsiderationData>,
    pub priority: types::ActionScore,
    pub action_key: String,
}
