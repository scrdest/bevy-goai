//! Type aliases and 'abstracting' newtypes.

use std::sync::Arc;

type ThreadSafeRefValue<T> = Arc<T>;

/// An abstraction over whatever thread-safe shared pointer type (i.e. Arc<T>-like) 
/// the library has decided to use.
#[derive(Debug, Hash, Default, bevy::reflect::Reflect)]
pub struct ThreadSafeRef<T: ?Sized> {
    wrapped: ThreadSafeRefValue<T>
}

impl<T> ThreadSafeRef<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self { wrapped: Arc::new(val) }
    }
}

impl<T: ?Sized> ThreadSafeRef<T> {
    #[inline]
    pub fn new_from_ref(val: ThreadSafeRefValue<T>) -> Self {
        Self { wrapped: val }
    }
}

impl<T: ?Sized> Clone for ThreadSafeRef<T> {
    fn clone(&self) -> Self {
        Self::new_from_ref(self.wrapped.clone())
    }
}

impl<T: ?Sized> From<Arc<T>> for ThreadSafeRef<T> {
    fn from(value: Arc<T>) -> Self {
        Self::new_from_ref(value)
    }
}

// Safety: these are wrappers o
unsafe impl<T: ?Sized + Sync + Send> Send for ThreadSafeRef<T> {}
unsafe impl<T: ?Sized + Sync + Send> Sync for ThreadSafeRef<T> {}

impl<T: ?Sized> std::ops::Deref for ThreadSafeRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.wrapped.deref()
    }
}

impl<T: ?Sized> AsRef<Arc<T>> for ThreadSafeRef<T> {
    fn as_ref(&self) -> &Arc<T> {
        &self.wrapped
    }
}

impl<T: PartialEq + ?Sized> PartialEq for ThreadSafeRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.wrapped == other.wrapped
    }
}

impl<T: Eq + ?Sized> Eq for ThreadSafeRef<T> {}

impl<T: PartialOrd + ?Sized> PartialOrd for ThreadSafeRef<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.wrapped.partial_cmp(&other.wrapped)
    }
}

impl<T: Ord + ?Sized> Ord for ThreadSafeRef<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.wrapped.cmp(&other.wrapped)
    }
}

/// 
pub type ContextFetcherKey = crate::identifiers::ContextFetcherIdentifier;
pub type UtilityCurveKey = String;

/// Type alias to make it easier to switch out what datatypes are used for Actions. 
/// Action Keys are effectively IDs, so they do not need to be human-readable.
pub type ActionKey = String;

pub type ActionScore = f32;

pub const MIN_CONSIDERATION_SCORE: ActionScore = 0.;
pub const MAX_CONSIDERATION_SCORE: ActionScore = 1.;

pub type ActionTemplate = crate::actions::ActionTemplate;
pub type ActionTemplateRef = ThreadSafeRef<ActionTemplate>;

pub type ActionContext = crate::actions::ActionContext;
pub type ActionContextRef = ActionContext; // currently Entity, which is Copy and serves as a reference copied.
pub type ActionContextList = Vec<ActionContextRef>;

// Type aliases - to express intent better.
pub type AiEntity = bevy::prelude::Entity;
pub type PawnEntity = bevy::prelude::Entity;
pub type PawnEntityRef = Option<PawnEntity>;

pub use crate::context_fetchers::ContextFetcherInputs;
pub use crate::context_fetchers::ContextFetcherOutputs;
pub use crate::context_fetchers::ContextFetcherSystem;
pub use crate::context_fetchers::IntoContextFetcherSystem;

pub use crate::considerations::ConsiderationInputs;
pub use crate::considerations::ConsiderationOutputs;
pub use crate::considerations::ConsiderationSystem;
pub use crate::considerations::IntoConsiderationSystem;

pub type SmartObject = String;

pub type ActionSetRef = SmartObject;
pub type ActionSetsRef = ThreadSafeRef<Vec<ActionSetRef>>;

pub type EntityIdentifier = crate::entity_identifier::EntityIdentifier;

pub type AiLodLevelPrimitive = u8;
