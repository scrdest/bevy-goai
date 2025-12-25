/// A convenient type wrapping Entity with an optional Name.
/// Acts as either an Entity or a Name for Display purposes based on what it holds. 
/// Otherwise acts as an Entity for any other purpose.
#[derive(Debug, Clone)]
pub enum EntityIdentifier {
    Entity(bevy::prelude::Entity),
    EntityAndName(bevy::prelude::Entity, String),
}

impl From<bevy::prelude::Entity> for EntityIdentifier {
    fn from(value: bevy::prelude::Entity) -> Self {
        Self::Entity(value)
    }
}

impl From<(bevy::prelude::Entity, String)> for EntityIdentifier {
    fn from(value: (bevy::prelude::Entity, String)) -> Self {
        Self::EntityAndName(value.0, value.1)
    }
}

impl Into<bevy::prelude::Entity> for EntityIdentifier {
    fn into(self) -> bevy::prelude::Entity {
        match self {
            Self::Entity(e) => e,
            Self::EntityAndName(e, _) => e,
        }
    }
}

impl std::ops::Deref for EntityIdentifier {
    type Target = bevy::prelude::Entity;
    
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Entity(e) => e,
            Self::EntityAndName(e, _) => e,
        }
    }
}

impl std::fmt::Display for EntityIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Entity(e) => e.fmt(f),
            Self::EntityAndName(_, s) => s.fmt(f)
        }
    }
}

impl std::hash::Hash for EntityIdentifier {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Entity(e) => e.hash(state),
            Self::EntityAndName(e, _) => e.hash(state)
        }
    }
}

impl PartialEq for EntityIdentifier {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Entity(l0), Self::Entity(r0)) => l0 == r0,
            (Self::EntityAndName(l0, _), Self::EntityAndName(r0, _)) => l0 == r0,
            _ => false,
        }
    }
}

impl Eq for EntityIdentifier {}
