use bevy::prelude::*;
use crate::actions::{ActionTemplate};

#[cfg(feature = "actionset_loader")]
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Reflect)]
#[cfg_attr(any(feature = "actionset_loader"), derive(Serialize, Deserialize, Asset))]
pub struct ActionSet {
    pub name: String,
    pub actions: crate::types::CraniumList<ActionTemplate>,
}

impl ActionSet {
    pub fn new<IS: Into<String>>(name: IS, actions: crate::types::CraniumList<ActionTemplate>) -> Self {
        Self {
            name: name.into(),
            actions: actions,
        }
    }
}
