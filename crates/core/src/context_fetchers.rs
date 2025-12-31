use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use bevy::prelude::*;
use crate::types::{self, ActionContext, AiEntity, PawnEntity};


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

/// Convenience type-alias for the output type required from a ContextFetcher System. 
pub type ContextFetcherOutputs = Vec<ActionContext>;

/// A specialization of Bevy's `System` trait (or more precisely, `ReadOnlySystem`) 
/// that can be used as a Cortex ContextFetcher.
/// 
/// Note that the associated `In` type only adds the restriction that your custom 
/// functions must *at least* accept the metadata piped into them; you can add any 
/// number of Queries, Resource accesses, etc. - as long as they are read-only.
pub trait ContextFetcherSystem: bevy::ecs::system::ReadOnlySystem<
    In = ContextFetcherInputs, 
    Out = ContextFetcherOutputs,
> {}

impl<
    ROS: bevy::ecs::system::ReadOnlySystem<
        In = ContextFetcherInputs, 
        Out = ContextFetcherOutputs,
    >
> ContextFetcherSystem for ROS {}


/// A specialization of Bevy's `IntoSystem` trait that defines any function 
/// that can be turned into a valid Cortex ContextFetcher System.
pub trait IntoContextFetcherSystem<Marker>: IntoSystem<
    ContextFetcherInputs, 
    ContextFetcherOutputs, 
    Marker,
> {}

impl<
    CS: ContextFetcherSystem, 
    Marker, 
    IS: IntoSystem<
        ContextFetcherInputs,
        ContextFetcherOutputs, 
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


/// Something that allows us to register a ContextFetcher to the World. 
/// 
/// Note that for convenience, the first registration attempt 
/// will initialize *an empty registry* if one does not exist yet, so
/// you don't need to use `app.initialize_resource::<UtilityCurveRegistry>()` 
/// unless you want to be explicit about it.
pub trait AcceptsContextFetcherRegistrations {
    fn register_context_fetcher<
        CS: ContextFetcherSystem, 
        Marker, 
        F: IntoContextFetcherSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: types::ContextFetcherKey,
    ) -> &mut Self;
}

impl AcceptsContextFetcherRegistrations for World {
    fn register_context_fetcher<
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

impl AcceptsContextFetcherRegistrations for App {
    fn register_context_fetcher<
        CS: ContextFetcherSystem, 
        Marker, 
        F: IntoContextFetcherSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: types::ContextFetcherKey,
    ) -> &mut Self {
        self.world_mut().register_context_fetcher(consideration, key);
        self
    }
}

