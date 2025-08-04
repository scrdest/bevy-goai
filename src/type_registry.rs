use std::borrow::Borrow;

use bevy::reflect::{TypeRegistry, TypeData};
use crate::errors::DynResolutionError;


pub trait TypeRegistryIdentifierBuildable: Sized {
    /// This is a helper trait for the TypeRegistryIdentifier family of traits.
    /// This is effectively an extension of From<String> for cases that may require a bit more work to construct.
    fn build_from_string(value: String) -> Self;
}

impl<T: From<String>> TypeRegistryIdentifierBuildable for T {
    /// Trivial constructors are blanket-implementable.
    fn build_from_string(value: String) -> Self {
        Self::from(value)
    }
}

pub trait TypeRegistryIdentifierRecoverable: Sized {
    /// This is a helper trait for the TypeRegistryIdentifier family of traits.
    /// This is effectively an extension of Borrow<&str> for cases that may require a bit more work to construct.
    fn as_identifier_string(&self) -> &str;
}

impl<T: Borrow<str>> TypeRegistryIdentifierRecoverable for T {
    /// Trivial constructors are blanket-implementable.
    fn as_identifier_string(&self) -> &str {
        self.borrow()
    }
}


pub trait TypeRegistryIdentifier: TypeRegistryIdentifierBuildable + TypeRegistryIdentifierRecoverable {
    /// This trait effectively represents a newtype wrapper holding a String that *provably* represents some *registered* type. 
    /// In other words, to get an instance of the String wrapper, we must pass a check-in with some type registry.

    fn from_string_identifier(value: String, registry: &TypeRegistry) -> Result<Self, DynResolutionError> {
        let retrieved = registry.get_with_type_path(&value);
        match retrieved {
            None => Err(DynResolutionError::NotInRegistry(value.to_owned())),
            Some(_) => Ok(Self::build_from_string(value))
        }
    }
}

pub trait TypeRegistryIdentifierFor<T: 'static + TypeData>: TypeRegistryIdentifier {
    /// This trait effectively represents a newtype wrapper holding a String that *provably* represents a SPECIFIC *registered* type. 
    /// This is more narrow than TypeRegistryIdentifier, as not only do we have to prove there is a registered type by that name, 
    /// but also that the type is what we're expecting.

    fn from_string_identifier_typechecked(value: String, registry: &TypeRegistry) -> Result<Self, DynResolutionError> {
        let retrieved = registry.get_with_type_path(&value).map(|tr| tr.contains::<T>());
        match retrieved {
            None => Err(DynResolutionError::NotInRegistry(value.to_owned())),
            Some(maybe_type) => match maybe_type {
                true => Ok(Self::build_from_string(value)),
                false => Err(DynResolutionError::UnexpectedType(value.to_owned())),
            }
        }
    }

    fn to_represented_type(&self, registry: &TypeRegistry) -> T;
}
