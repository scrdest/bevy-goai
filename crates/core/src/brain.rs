use bevy::prelude::*;
use crate::types::CortexKvMap;

#[derive(Component)]
pub struct Relationships(CortexKvMap<Entity, CortexKvMap<String, f32>>);


#[derive(Component)]
pub struct Personality(CortexKvMap<String, f32>);
