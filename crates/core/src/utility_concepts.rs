use std::borrow::Borrow;
use bevy::prelude::*;
use bevy::reflect::{Reflect};
use serde::{Serialize, Deserialize};


#[derive(Reflect, Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(transparent)]
pub struct ContextFetcherIdentifier(pub String);

impl ContextFetcherIdentifier {
    pub fn from_string(value: String) -> Self {
        Self(value)
    }
}

impl<IS: Into<String>> From<IS> for ContextFetcherIdentifier {
    fn from(value: IS) -> Self {
        Self::from_string(value.into())
    }
}

impl Borrow<str> for ContextFetcherIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Borrow<str> for &ContextFetcherIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Borrow<String> for ContextFetcherIdentifier {
    fn borrow(&self) -> &String {
        self.0.borrow()
    }
}

impl Borrow<String> for &ContextFetcherIdentifier {
    fn borrow(&self) -> &String {
        self.0.borrow()
    }
}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(transparent)]
pub struct CurveIdentifier(String);

impl CurveIdentifier {
    pub fn from_string(value: String) -> Self {
        Self(value)
    }
}

impl<IS: Into<String>> From<IS> for CurveIdentifier {
    fn from(value: IS) -> Self {
        Self::from_string(value.into())
    }
}

impl Borrow<str> for CurveIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Borrow<str> for &CurveIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Borrow<String> for CurveIdentifier {
    fn borrow(&self) -> &String {
        self.0.borrow()
    }
}

impl Borrow<String> for &CurveIdentifier {
    fn borrow(&self) -> &String {
        self.0.borrow()
    }
}


#[derive(Reflect, Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
#[serde(transparent)]
pub struct ConsiderationIdentifier(String);


impl ConsiderationIdentifier {
    pub fn from_string(value: String) -> Self {
        Self(value)
    }
}

impl<IS: Into<String>> From<IS> for ConsiderationIdentifier {
    fn from(value: IS) -> Self {
        Self::from_string(value.into())
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
