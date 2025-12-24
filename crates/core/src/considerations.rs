use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::types;
use crate::utility_concepts::{ConsiderationIdentifier, CurveIdentifier};


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
pub struct ConsiderationData {
    #[serde(rename="consideration")]
    pub func_name: ConsiderationIdentifier,

    #[serde(rename="curve")]
    pub curve_name: CurveIdentifier,

    pub min: types::ActionScore,
    pub max: types::ActionScore,
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


#[derive(Clone, Debug)]
pub struct ConsiderationMappedToSystemIds {
    pub func_name: ConsiderationIdentifier,

    // NOTE: This is Result<T> as the registry lookup is fallible 
    //       and we will need to propagate these errors later on.
    pub consideration_systemid: Result<types::ConsiderationSignature, ()>,
    pub curve_name: CurveIdentifier,

    pub min: types::ActionScore,
    pub max: types::ActionScore,
}


/// A Message that represents a request to the user code to run a batch of Consideration Systems (and
/// their Curves) corresponding to keys provided. It should return ConsiderationResponse Messages for
/// any associated Action IF it has any chance of getting picked (i.e. you can optimize away some
/// Messages if you know the resulting score is too low).
/// 
/// This is a layer of abstraction between the core AI engine and user code; 
/// the AI code does not care how exactly you calculate the scores for the 
/// response, just that you get it done somehow. 
/// 
/// The idea is that as a user, you route the messages to your own implementations as appropriate.
/// Because we use abstracted Messages for communication, any number of libraries and systems can 
/// hook into this core engine, and AI commands remain valid across saves and version migrations 
/// (with the worst-case scenario being that the app no longer supports the selected Action and the 
/// message is ignored).
/// 
/// We batch Requests because Considerations are not entirely independent.
/// 
/// The first reason is the scoring adjustment. 
/// A quirk of how we do Utility math means that Considerations are subtractive; each added Consideration 
/// is another thing dragging the total score down a bit. That disincentivizes making AIs smarter; no bueno.
/// There is a math hack to work around that, but it relies on knowing how many Considerations we've used.
/// 
/// The second reason is optimization.
/// If we know this Action is never gonna make it, we would ideally avoid running any more Considerations 
/// for it - Considerations can be fairly complex and expensive queries, so the fewer, the better.
#[derive(Message, Debug, Clone)]
pub struct BatchedConsiderationRequest {
    pub entity: Entity, 
    pub scored_action_template: types::ActionTemplate,
    pub scored_context: types::ActionContext,
    /// To cheaply reference ScoredContext - tuple of (MsgId, CtxIdx)
    pub scored_context_index: (usize, usize),
    pub considerations: Vec<ConsiderationMappedToSystemIds>,
}

/// A Message that represents a user-code response to a ConsiderationRequest.
/// 
/// The expected flow is that library users read ConsiderationRequest messages 
/// in their apps and write back ConsiderationResponse messages to the engine.
/// 
/// This is a layer of abstraction between the core AI engine and user code; 
/// the AI code does not care how exactly you calculate the score for the response, 
/// just that you get it done somehow.
#[derive(Message, Debug)]
pub struct ConsiderationResponse {
    pub entity: Entity, 
    pub scored_action_template: types::ActionTemplate,
    pub scored_context: types::ActionContext,
    pub score: types::ActionScore,
}
