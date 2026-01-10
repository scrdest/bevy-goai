use bevy::prelude::*;
use crate::ai::*;
use crate::memories::{Memories};

// An AI 
#[derive(Component)]
struct Senses(crate::types::CortexList<Box<dyn Sense>>);


trait Sense: Send + Sync {
    fn update(&self, memories: &mut Memories, world: &World);
}

struct VisionSense {
    range: f32,
}

impl Sense for VisionSense {
    fn update(&self, memories: &mut Memories, world: &World) {
        // Query visible entities and update memories
        
    }
}

// Sense system
fn update_senses(
    mut query: Query<(&AIController, &Senses, &mut Memories)>,
    world: &World,
) {
    for (_, senses, mut memories) in query.iter_mut() {
        for sense in senses.0.iter() {
            sense.update(&mut memories, world);
        }
    }
}

