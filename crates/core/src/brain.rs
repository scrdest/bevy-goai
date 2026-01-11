use bevy::prelude::*;
use crate::types::CraniumKvMap;

#[derive(Component)]
pub struct Relationships(CraniumKvMap<Entity, CraniumKvMap<String, f32>>);


#[derive(Component)]
pub struct Personality(CraniumKvMap<String, f32>);
