use std::collections::HashMap;
use bevy::prelude::*;

#[derive(Component)]
pub struct Relationships(HashMap<Entity, HashMap<String, f32>>);


#[derive(Component)]
pub struct Personality(HashMap<String, f32>);
