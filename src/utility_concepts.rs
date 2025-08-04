use std::borrow::Borrow;
use bevy::prelude::*;
use bevy::reflect::{DynamicList, Reflect};
use serde::{Serialize, Deserialize};
use crate::type_registry::TypeRegistryIdentifier;
use crate::pawn::Pawn;

pub trait CurveFunc: Reflect {
    fn run_curve(normalized_val: f32) -> f32;
}

pub trait ContextFetcher: Reflect {
    fn fetch_contexts(
        pawn: &Pawn, 
        world: &World,
    ) -> DynamicList;
}

pub trait Consideration: Reflect {
    fn run_consideration(
        pawn: &Pawn, 
        world: &World,
    ) -> f32;
}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct ContextFetcherIdentifier(String);

impl From<String> for ContextFetcherIdentifier {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Borrow<str> for ContextFetcherIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl TypeRegistryIdentifier for ContextFetcherIdentifier {}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct CurveIdentifier(String);

impl From<String> for CurveIdentifier {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Borrow<str> for CurveIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl TypeRegistryIdentifier for CurveIdentifier {}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct ConsiderationIdentifier(String);

impl From<String> for ConsiderationIdentifier {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Borrow<str> for ConsiderationIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl TypeRegistryIdentifier for ConsiderationIdentifier {}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
struct ConsiderationAsset {
    min: f32,
    max: f32,
    function: ConsiderationIdentifier, 
    curve: CurveIdentifier, 
}
