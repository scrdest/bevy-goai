use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use bevy::prelude::*;
use crate::actions::*;
use crate::types::{self, ActionScore, ActionContext, AiEntity, PawnEntity};

/// A Message representing a single request for a ContextFetcher call from the AI lib to user code. 
/// 
/// Users are expected to implement a System that uses a MessageReader for this type and dispatches 
/// their custom logic to handle them on a case-by-case basis..
#[derive(Message, Debug)]
pub struct ContextFetcherRequest {
    pub action_template: Arc<ActionTemplate>,
    pub audience: types::EntityIdentifier, 
}

impl ContextFetcherRequest {
    pub fn new(
        audience: types::EntityIdentifier, 
        action_template: Arc<ActionTemplate>,
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
    pub action_template: Arc<ActionTemplate>, 

    /// The AI this was requested for; primarily so that we can split 
    /// the scoring process per each Audience, 
    /// even if the Messages for them wind up interleaved.
    pub audience: types::EntityIdentifier, 
}

impl ContextFetchResponse {
    pub fn new(
        action_template: Arc<ActionTemplate>,
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


/// Convenience type-alias for generic inputs piped into each ContextFetcher. 
/// 
/// You can use it to simply write `fn your_context_fetcher(params: ContextFetcherInputs, your_query:...`
/// instead of having to memorize the specific interface required by the library.
/// 
/// This currently comprises:
/// - Requesting AI - as Entity
/// - Requesting AI's Pawn (controlled Entity) - as Entity
/// 
/// Changes to this interface will be considered semver-breaking once the core lib stabilizes.
/// 
/// Note that ContextFetchers are Plain Old (Read-Only!) Systems, so you can use `Queries`, 
/// `Resources` and all the other Bevy goodness to write your ContextFetcher logic - but you 
/// must also include these inputs as a parameter.
/// 
/// The point of those inputs is to let the World inject metadata about the AI query 
/// into your ContextFetchers so that you can use them in your own logic. 
/// 
/// The key Entities in play in particular are included to enable no-fuss fast 
/// retrieval of data about them in your custom Queries (using `Query::get()`). 
pub type ContextFetcherInputs = bevy::prelude::In<(
    AiEntity, 
    PawnEntity,
)>;

pub trait ContextFetcherSystem: bevy::ecs::system::ReadOnlySystem<
    In = ContextFetcherInputs, 
    Out = ActionContext,
> {}

impl<
    ROS: bevy::ecs::system::ReadOnlySystem<
        In = ContextFetcherInputs, 
        Out = ActionContext
    >
> ContextFetcherSystem for ROS {}

pub trait IntoContextFetcherSystem<Marker>: IntoSystem<
    ContextFetcherInputs, 
    ActionScore, 
    Marker,
> {}

impl<
    CS: ContextFetcherSystem, 
    Marker, 
    IS: IntoSystem<
        ContextFetcherInputs,
        ActionScore, 
        Marker,
        System = CS
    >
> IntoContextFetcherSystem<Marker> for IS {}


#[derive(Clone)]
pub struct ContextFetcherMappedToSystem {
    pub context_fetcher_system: Result<Arc<RwLock<dyn ContextFetcherSystem>>, ()>,
}

#[derive(Resource, Default)]
pub struct ContextFetcherKeyToSystemMap {
    pub mapping: HashMap<
        types::ContextFetcherKey, 
        std::sync::Arc<std::sync::RwLock<dyn ContextFetcherSystem>>
    >
}


pub trait StoresContextFetcherRegistrations {
    fn register_consideration<
        CS: ContextFetcherSystem, 
        Marker, 
        F: IntoContextFetcherSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: types::ContextFetcherKey,
    ) -> &mut Self;
}

impl StoresContextFetcherRegistrations for World {
    fn register_consideration<
        CS: ContextFetcherSystem, 
        Marker, 
        F: IntoContextFetcherSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: types::ContextFetcherKey,
    ) -> &mut Self {
        let mut system = F::into_system(consideration);
        system.initialize(self);
        let mut system_registry = self.get_resource_or_init::<ContextFetcherKeyToSystemMap>();
        system_registry.mapping.insert(
            key, 
            std::sync::Arc::new(std::sync::RwLock::new(
                system
            )));
        self
    }
}

impl StoresContextFetcherRegistrations for &mut App {
    fn register_consideration<
        CS: ContextFetcherSystem, 
        Marker, 
        F: IntoContextFetcherSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: types::ContextFetcherKey,
    ) -> &mut Self {
        self.world_mut().register_consideration(consideration, key);
        self
    }
}

