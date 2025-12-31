use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::types::{self, ActionScore, ActionContext, AiEntity, PawnEntity};
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

/// Convenience type-alias for the output type a Consideration must return.
pub type ConsiderationOutputs = ActionScore;


/// A specialization of Bevy's `System` trait (or more precisely, `ReadOnlySystem`) 
/// that can be used as a Cortex Consideration.
/// 
/// Note that the associated `In` type only adds the restriction that your custom 
/// functions must *at least* accept the metadata piped into them; you can add any 
/// number of Queries, Resource accesses, etc. - as long as they are read-only.
pub trait ConsiderationSystem: bevy::ecs::system::ReadOnlySystem<
    In = ConsiderationInputs, 
    Out = ActionScore
> {}

impl<
    ROS: bevy::ecs::system::ReadOnlySystem<
        In = ConsiderationInputs, 
        Out = ActionScore
    >
> ConsiderationSystem for ROS {}


/// A specialization of Bevy's `IntoSystem` trait that defines any function 
/// that can be turned into a valid Cortex Consideration System.
pub trait IntoConsiderationSystem<Marker>: IntoSystem<
    ConsiderationInputs, 
    ActionScore, 
    Marker,
> {}

impl<
    Marker, 
    CS: ConsiderationSystem, 
    IS: IntoSystem<
        ConsiderationInputs,
        ActionScore, 
        Marker,
        System = CS
    >
> IntoConsiderationSystem<Marker> for IS {}


#[derive(Clone)]
pub struct ConsiderationMappedToSystem {
    pub func_name: ConsiderationIdentifier,

    // NOTE: This is Result<T> as the registry lookup is fallible 
    //       and we will need to propagate these errors later on.
    // pub consideration_systemid: Result<types::ConsiderationSignature, ()>,
    pub consideration_system: Result<Arc<RwLock<dyn ConsiderationSystem>>, ()>,
    pub curve_name: CurveIdentifier,

    pub min: types::ActionScore,
    pub max: types::ActionScore,
}


#[derive(Resource, Default)]
pub struct ConsiderationKeyToSystemMap {
    pub mapping: HashMap<
        ConsiderationIdentifier, 
        std::sync::Arc<std::sync::RwLock<dyn ConsiderationSystem>>
    >
}


/// Something that allows us to register a ContextFetcher to the World. 
/// 
/// Note that for convenience, the first registration attempt 
/// will initialize *an empty registry* if one does not exist yet, so
/// you don't need to use `app.initialize_resource::<UtilityCurveRegistry>()` 
/// unless you want to be explicit about it.
pub trait AcceptsConsiderationRegistrations {
    fn register_consideration<
        CS: ConsiderationSystem, 
        Marker, 
        F: IntoConsiderationSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: ConsiderationIdentifier
    ) -> &mut Self;
}

impl AcceptsConsiderationRegistrations for World {
    fn register_consideration<
        CS: ConsiderationSystem, 
        Marker, 
        F: IntoConsiderationSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: ConsiderationIdentifier
    ) -> &mut Self {
        let mut system = F::into_system(consideration);
        system.initialize(self);
        let mut system_registry = self.get_resource_or_init::<ConsiderationKeyToSystemMap>();
        system_registry.mapping.insert(
            key, 
            std::sync::Arc::new(std::sync::RwLock::new(
                system
            )));
        self
    }
}

impl AcceptsConsiderationRegistrations for App {
    fn register_consideration<
        CS: ConsiderationSystem, 
        Marker, 
        F: IntoConsiderationSystem<Marker, System = CS> + 'static
    >(
        &mut self, 
        consideration: F, 
        key: ConsiderationIdentifier
    ) -> &mut Self {
        self.world_mut().register_consideration(consideration, key);
        self
    }
}
