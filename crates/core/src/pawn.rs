use bevy::prelude::*;
use crate::types::{PawnEntity, PawnEntityRef};

// An AIController on its own is just a jumped-up abstract decision process.
// To get actual NPCs, it needs to 'drive' another Entity - that Entity is the AI's Pawn.
// Note that this can be a classic NPC, but also things like squads, factions, or 'the world' for Director AI.
#[derive(Component, Clone, Default)]
pub struct Pawn(Option<PawnEntity>);

impl Pawn {
    pub fn new(maybe_pawn: Option<PawnEntity>) -> Self {
        Self(maybe_pawn)
    }

    pub const fn new_empty() -> Self {
        Self(None)
    }

    pub fn new_populated(pawn: PawnEntity) -> Self {
        Self(Some(pawn))
    }

    pub fn as_entity(&self) -> Option<&PawnEntity> {
        match &self.0 {
            None => None,
            Some(pawn_id) => Some(pawn_id)
        }
    }

    pub fn to_entity(self) -> Option<PawnEntity> {
        match self.0 {
            None => None,
            Some(pawn_id) => Some(pawn_id)
        }
    }
}

impl std::borrow::Borrow<PawnEntityRef> for Pawn {
    fn borrow(&self) -> &PawnEntityRef {
        &self.0
    }
}
