//! Identifiers for key types.
//! 
//! These are, broadly speaking, simple newtype wrappers whose main purpose is to 
//! future-proof the library and allow for the implementations of assorted Traits 
//! that will not 'leak' into the underlying, wrapped type.
//! 
//! Barring exceptional circumstances, all identifiers will be cheap and easy to 
//! convert into at least a reference to their respective underlying types.

use std::borrow::Borrow;

use bevy::prelude::*;
use bevy::reflect::{Reflect};

#[cfg(any(feature = "actionset_loader"))]
use serde::{Serialize, Deserialize};


#[derive(Reflect, Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(any(feature = "actionset_loader"), derive(Serialize, Deserialize))]
#[cfg_attr(any(feature = "actionset_loader"), serde(transparent))]
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


#[derive(Reflect, Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(any(feature = "actionset_loader"), derive(Serialize, Deserialize))]
#[cfg_attr(any(feature = "actionset_loader"), serde(transparent))]
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


#[derive(Reflect, Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(any(feature = "actionset_loader"), derive(Serialize, Deserialize))]
#[cfg_attr(any(feature = "actionset_loader"), serde(transparent))]
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
