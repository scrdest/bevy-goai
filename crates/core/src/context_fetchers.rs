use bevy::prelude::*;
use crate::actions::*;
use crate::types;

/// A Message representing a single request for a ContextFetcher call from the AI lib to user code. 
/// 
/// Users are expected to implement a System that uses a MessageReader for this type and dispatches 
/// their custom logic to handle them on a case-by-case basis..
#[derive(Message, Debug)]
pub struct ContextFetcherRequest {
    pub action_template: ActionTemplate,
    pub audience: types::EntityIdentifier, 
}

impl ContextFetcherRequest {
    pub fn new(
        audience: types::EntityIdentifier, 
        action_template: ActionTemplate,
    ) -> Self {
        Self {
            action_template: action_template,
            audience: audience,
        }
    }
}

#[derive(Message, Debug)]
pub struct ContextFetchResponse {
    /// The meat of the response - the Context that has been requested.
    pub contexts: types::ActionContextList, 
    
    /// The ActionTemplate this request came for (mainly to tie it back together as an Action)
    pub action_template: ActionTemplate, 

    /// The AI this was requested for; primarily so that we can split 
    /// the scoring process per each Audience, 
    /// even if the Messages for them wind up interleaved.
    pub audience: types::EntityIdentifier, 
}

impl ContextFetchResponse {
    pub fn new(
        action_template: ActionTemplate,
        contexts: types::ActionContextList,
        audience: types::EntityIdentifier,
    ) -> Self {
        Self {
            action_template: action_template,
            contexts: contexts,
            audience: audience,
        }
    }
}