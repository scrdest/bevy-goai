use bevy::prelude::*;

// An AIController on its own is just a jumped-up abstract decision process loop.
// To get actual NPCs, it needs to 'drive' another Entity - that Entity is the AI's Pawn.
// Note that this can be a classic NPC, but also things like squads, factions, or 'the world' for Director AI.
#[derive(Component)]
pub struct Pawn(Entity);
