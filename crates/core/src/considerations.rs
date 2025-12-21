use bevy::prelude::*;
use crate::types;
use crate::utility_concepts::{ConsiderationIdentifier, CurveIdentifier};

/// A Message that represents a request to the user code to run a Consideration System 
/// (plus Curve) corresponding to keys provided and return a ConsiderationResponse Message 
/// if the associated Action has any chance of getting picked.
/// 
/// This is a layer of abstraction between the core AI engine and user code; 
/// the AI code does not care how exactly you calculate the score for the response, 
/// just that you get it done somehow.
#[derive(Message, Debug)]
pub struct ConsiderationRequest {
    pub entity: Entity, 
    pub scored_action_template: types::ActionTemplate,
    pub scored_context: types::ActionContext,
    pub consideration_key: ConsiderationIdentifier,
    pub curve_key: CurveIdentifier,
    pub min: types::ActionScore,
    pub max: types::ActionScore,
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
    pub name: ConsiderationIdentifier, 
    pub entity: Entity, 
    pub scored_action_template: types::ActionTemplate,
    pub scored_context: types::ActionContext,
    pub score: types::ActionScore,
}
