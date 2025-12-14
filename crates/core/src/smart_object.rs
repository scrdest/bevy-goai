use bevy::prelude::*;
use crate::actionset::ActionSet;

// The overall design looks like this:
// 1) Each AI has 0+ (though practically speaking 1+) SmartObjects associated with it at a given moment.
// 
// 2) The SmartObjects are added and removed to AIs dynamically, based on the environment (e.g. for NPC AI, 
//    this is based on the items in the 'general proximity' of the Pawn, whatever that means).
//    
//    At the library level, we don't care what those rules are - downstream applications are free to specify 
//    their own Systems to add and remove SOs at their heart's content, as this is highly contextual.
//
// 3) SmartObjects are 'marketing' containers for ActionSets, consumed by AI Controllers. 
//    Something is a SO if it provides an AI with an ActionSet, based on some predicate (including 'always true'), 
//    by definition (i.e. anything that does that is an SO, even if we didn't call it that).
//
// 4) ActionSets are hot-reloadable Assets.
// 
// 5) Therefore, we cannot store the ActionSets raw. Instead, we store a key of the ActionSet.
// 
// 6) ...but we still need to be able to recover 'em later as data - so we store them in a HashMap Resource.
//
// 7) Therefore the flow for processing Actions in the AI goes: 
//    AI -> SmartObject component key -> Res<ActionSetStore> lookup -> ActionSet -> <Actions>
//
//    And the Asset flow goes:
//    File (re)load -> Asset<ActionSet> -> ResMut<ActionSetStore> -> Upsert key ActionSet.name with a *clone* of the Asset's wrapped ActionSet.


#[derive(Resource, Default, Reflect)]
pub struct ActionSetStore {
    pub map_by_name: std::collections::HashMap<String, ActionSet>
}


#[derive(Component, Default, Reflect)]
pub struct SmartObjects {
    pub actionset_refs: Vec<String>
}
