//! Type aliases and 'abstracting' newtypes.

use std::sync::Arc;

/// 
pub type ContextFetcherKey = crate::utility_concepts::ContextFetcherIdentifier;

/// Type alias to make it easier to switch out what datatypes are used for Actions. 
/// Action Keys are effectively IDs, so they do not need to be human-readable.
pub type ActionKey = String;

pub type ActionScore = f32;

pub const MIN_CONSIDERATION_SCORE: ActionScore = 0.;
pub const MAX_CONSIDERATION_SCORE: ActionScore = 1.;

pub type ActionTemplate = crate::actions::ActionTemplate;
pub type ActionTemplateRef = Arc<ActionTemplate>;

pub type ActionContext = crate::actions::ActionContext;
pub type ActionContextRef = Arc<ActionContext>;
pub type ActionContextList = Vec<ActionContextRef>;

// Type aliases - to express intent better.
pub type AiEntity = bevy::prelude::Entity;
pub type PawnEntity = bevy::prelude::Entity;

/// Convenience type-alias for generic inputs piped into each Consideration. 
/// 
/// You can use it to simply write `fn your_consideration(params: ConsiderationInputs, your_query:...`
/// instead of having to memorize the specific interface required by the library.
/// 
/// This currently comprises:
/// - Requesting AI - as Entity
/// - Requesting AI's Pawn (controlled Entity) - as Entity
/// - The full Context this Consideration is scoring
/// 
/// Changes to this interface will be considered semver-breaking once the core lib stabilizes.
/// 
/// Note that Considerations are Plain Old Systems, so you can use `Query`, `Commands`, 
/// and all the other Bevy goodness to write your Consideration logic - but you must 
/// also include these inputs as a parameter.
/// 
/// The point of those inputs is to let the World inject metadata about the AI query 
/// into your Considerations so that you can use them in your own logic. 
/// 
/// The key Entities in play in particular are included to enable no-fuss fast 
/// retrieval of data about them in your custom Queries (using `Query::get()`). 
pub type ConsiderationInputs = bevy::prelude::In<(
    AiEntity, 
    PawnEntity,
    Arc<ActionContext>, 
)>;

/// A general interface for any Consideration.
/// 
/// Considerations are, generally, user-implemented Systems. 
/// They can do anything you want (run queries, read resources, etc.); this interface only cares 
/// about the return value and inputs piped into each Consideration (i.e. the In<Whatever> params).
pub type ConsiderationSignature = bevy::ecs::system::SystemId<
    // Input(s):
    ConsiderationInputs,
    // Output:
    ActionScore,
>;

// pub(crate) type Action = crate::actions::Action;

pub type EntityIdentifier = crate::entity_identifier::EntityIdentifier;

pub type AiLodLevelPrimitive = u8;
