//! Type aliases and 'abstracting' newtypes.

/// 
pub type ContextFetcherKey = crate::utility_concepts::ContextFetcherIdentifier;

/// Type alias to make it easier to switch out what datatypes are used for Actions. 
/// Action Keys are effectively IDs, so they do not need to be human-readable.
pub type ActionKey = String;

pub type ActionScore = f32;

pub const MIN_CONSIDERATION_SCORE: ActionScore = 0.;
pub const MAX_CONSIDERATION_SCORE: ActionScore = 1.;

pub type ActionTemplate = crate::actions::ActionTemplate;
pub type ActionContext = crate::actions::ActionContext;
pub type ActionContextList = Vec<ActionContext>;

/// A general interface for any Consideration.
/// 
/// Considerations are, generally, user-implemented Systems. 
/// They can do anything you want (run queries, read resources, etc.); this interface only cares 
/// about the return value and inputs piped into each Consideration (i.e. the In<Whatever> params).
pub type ConsiderationSignature = bevy::ecs::system::SystemId<
    // Input(s):
    (),
    // Output:
    ActionScore,
>;

// pub(crate) type Action = crate::actions::Action;

pub type EntityIdentifier = crate::entity_identifier::EntityIdentifier;
