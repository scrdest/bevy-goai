//! Type aliases and 'abstracting' newtypes.

/// 
pub type ContextFetcherKey = crate::utility_concepts::ContextFetcherIdentifier;

/// Type alias to make it easier to switch out what datatypes are used for Actions. 
/// Action Keys are effectively IDs, so they do not need to be human-readable.
pub type ActionKey = String;

pub type ActionScore = f32;

pub type ActionTemplate = crate::actions::ActionTemplate;
pub type ActionContext = crate::actions::ActionContext;
pub type ActionContextList = Vec<ActionContext>;

// pub(crate) type Action = crate::actions::Action;
