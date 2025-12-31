//! Type aliases and 'abstracting' newtypes.

use std::sync::Arc;

/// 
pub type ContextFetcherKey = crate::utility_concepts::ContextFetcherIdentifier;
pub type UtilityCurveKey = String;

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

pub use crate::context_fetchers::ContextFetcherInputs;
pub use crate::context_fetchers::ContextFetcherOutputs;
pub use crate::context_fetchers::ContextFetcherSystem;
pub use crate::context_fetchers::IntoContextFetcherSystem;

pub use crate::considerations::ConsiderationInputs;
pub use crate::considerations::ConsiderationOutputs;
pub use crate::considerations::ConsiderationSystem;
pub use crate::considerations::IntoConsiderationSystem;

pub type EntityIdentifier = crate::entity_identifier::EntityIdentifier;

pub type AiLodLevelPrimitive = u8;
