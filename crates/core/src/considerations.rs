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

pub type ConsiderationOutputs = ActionScore;

/// A general interface for any Consideration.
/// 
/// Considerations are, generally, user-implemented Systems. 
/// They can do anything you want (run queries, read resources, etc.); this interface only cares 
/// about the return value and inputs piped into each Consideration (i.e. the In<Whatever> params).
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

/// Something that can be turned into a Consideration. 
/// Generally meant for functions with a compatible interface.
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


/// A Message that represents a request to the user code to run a batch of Consideration Systems (and
/// their Curves) corresponding to keys provided. It should return ConsiderationResponse Messages for
/// any associated Action IF it has any chance of getting picked (i.e. you can optimize away some
/// Messages if you know the resulting score is too low).
/// 
/// This is a layer of abstraction between the core AI engine and user code; 
/// the AI code does not care how exactly you calculate the scores for the 
/// response, just that you get it done somehow. 
/// 
/// The idea is that as a user, you route the messages to your own implementations as appropriate.
/// Because we use abstracted Messages for communication, any number of libraries and systems can 
/// hook into this core engine, and AI commands remain valid across saves and version migrations 
/// (with the worst-case scenario being that the app no longer supports the selected Action and the 
/// message is ignored).
/// 
/// We batch Requests because Considerations are not entirely independent.
/// 
/// The first reason is the scoring adjustment. 
/// A quirk of how we do Utility math means that Considerations are subtractive; each added Consideration 
/// is another thing dragging the total score down a bit. That disincentivizes making AIs smarter; no bueno.
/// There is a math hack to work around that, but it relies on knowing how many Considerations we've used.
/// 
/// The second reason is optimization.
/// If we know this Action is never gonna make it, we would ideally avoid running any more Considerations 
/// for it - Considerations can be fairly complex and expensive queries, so the fewer, the better.
#[derive(Message, Clone)]
pub struct BatchedConsiderationRequest {
    pub entity: types::EntityIdentifier, 
    pub scored_action_template: types::ActionTemplateRef,
    pub scored_context: types::ActionContextRef,
    pub considerations: Vec<ConsiderationMappedToSystem>,
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
