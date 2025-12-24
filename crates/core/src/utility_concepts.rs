use std::borrow::Borrow;
use bevy::prelude::*;
use bevy::reflect::{Reflect};
use serde::{Serialize, Deserialize};


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct ContextFetcherIdentifier(pub String);

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


#[derive(Reflect, Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
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

impl std::fmt::Display for ConsiderationIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug)]
struct ConsiderationAsset {
    min: f32,
    max: f32,
    function: ConsiderationIdentifier, 
    curve: CurveIdentifier, 
}
