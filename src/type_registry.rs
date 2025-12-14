use std::borrow::Borrow;

use bevy::prelude::*;
use bevy::{ecs::reflect::AppFunctionRegistry, reflect::{func::FunctionRegistry, TypeRegistry}};
use serde::{Deserialize, Serialize};
use crate::errors::DynResolutionError;


pub trait TypeRegistryIdentifierBuildable: Sized {
    /// This is a helper trait for the IsTypeRegistryIdentifier family of traits.
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
    /// This is a helper trait for the IsTypeRegistryIdentifier family of traits.
    /// This is effectively an extension of Borrow<&str> for cases that may require a bit more work to construct.
    fn as_identifier_string(&self) -> &str;
}

impl<T: Borrow<str>> TypeRegistryIdentifierRecoverable for T {
    /// Trivial constructors are blanket-implementable.
    fn as_identifier_string(&self) -> &str {
        self.borrow()
    }
}

pub enum ReflectTypeRegistry<'a> {
    Type(&'a TypeRegistry),
    Func(&'a FunctionRegistry),
    AppType(Res<'a, AppTypeRegistry>),
    AppFunc(Res<'a, AppFunctionRegistry>),
}

impl<'a> From<&'a TypeRegistry> for ReflectTypeRegistry<'a> {
    fn from(value: &'a TypeRegistry) -> Self {
        Self::Type(value)
    }
}

impl<'a> From<&'a FunctionRegistry> for ReflectTypeRegistry<'a> {
    fn from(value: &'a FunctionRegistry) -> Self {
        Self::Func(value)
    }
}

impl<'a> From<Res<'a, AppTypeRegistry>> for ReflectTypeRegistry<'a> {
    fn from(value: Res<'a, AppTypeRegistry>) -> Self {
        Self::AppType(value)
    }
}

impl<'a> From<Res<'a, AppFunctionRegistry>> for ReflectTypeRegistry<'a> {
    fn from(value: Res<'a, AppFunctionRegistry>) -> Self {
        Self::AppFunc(value)
    }
}

#[derive(Reflect, Serialize, Deserialize, Debug, Clone)]
pub struct TypeRegistryTypeIdentifier(pub(crate) String); 

impl Borrow<str> for TypeRegistryTypeIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}


#[derive(Reflect, Serialize, Deserialize, Debug, Clone)]
pub struct TypeRegistryFuncIdentifier(String);

impl Borrow<str> for TypeRegistryFuncIdentifier {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}



#[derive(Reflect, Serialize, Deserialize, Debug, Clone)]
pub enum TypeRegistryIdentifier{
    Type(TypeRegistryTypeIdentifier),
    AppType(TypeRegistryTypeIdentifier),
    Func(TypeRegistryFuncIdentifier),
    AppFunc(TypeRegistryFuncIdentifier),
}

impl TypeRegistryIdentifier {
    pub fn is_any_type(&self) -> bool {
        match self {
            TypeRegistryIdentifier::Type(_) => true,
            TypeRegistryIdentifier::AppType(_) => true,
            _ => false,
        }
    }

    pub fn is_any_func(&self) -> bool {
        match self {
            TypeRegistryIdentifier::Func(_) => true,
            TypeRegistryIdentifier::AppFunc(_) => true,
            _ => false,
        }
    }

    pub fn is_app_func(&self) -> bool {
        match self {
            TypeRegistryIdentifier::AppFunc(_) => true,
            _ => false,
        }
    }

    pub fn is_nonapp_func(&self) -> bool {
        match self {
            TypeRegistryIdentifier::Func(_) => true,
            _ => false,
        }
    }

    pub fn try_to_any_type(self) -> Option<TypeRegistryTypeIdentifier> {
        match self {
            TypeRegistryIdentifier::Type(x) => Some(x),
            TypeRegistryIdentifier::AppType(x) => Some(x),
            _ => None
        }
    }

    pub fn try_to_any_func(self) -> Option<TypeRegistryFuncIdentifier> {
        match self {
            TypeRegistryIdentifier::Func(x) => Some(x),
            TypeRegistryIdentifier::AppFunc(x) => Some(x),
            _ => None
        }
    }
}

impl Borrow<str> for TypeRegistryIdentifier {
    fn borrow(&self) -> &str {
        match self {
            Self::Type(x) => x.borrow(),
            Self::AppType(x) => x.borrow(),
            Self::Func(x) => x.borrow(),
            Self::AppFunc(x) => x.borrow(),
        }
    }
}

impl Borrow<str> for &TypeRegistryIdentifier {
    fn borrow(&self) -> &str {
        match self {
            TypeRegistryIdentifier::Type(x) => x.borrow(),
            TypeRegistryIdentifier::AppType(x) => x.borrow(),
            TypeRegistryIdentifier::Func(x) => x.borrow(),
            TypeRegistryIdentifier::AppFunc(x) => x.borrow(),
        }
    }
}


pub trait IsTypeRegistryIdentifier: TypeRegistryIdentifierRecoverable {
    /// This trait effectively represents a newtype wrapper holding a String that *provably* represents some *registered* type. 
    /// In other words, to get an instance of the String wrapper, we must pass a check-in with some type registry.

    fn from_string_identifier(value: String, registry: &ReflectTypeRegistry) -> Result<TypeRegistryIdentifier, DynResolutionError> {
        let retrieved = match registry {
            // (IsType, IsApp, HasRegistry) triples
            ReflectTypeRegistry::Type(type_registry) => (true, false, type_registry.get_with_type_path(&value).is_some()),
            ReflectTypeRegistry::AppType(type_registry) => (true, true, type_registry.read().get_with_type_path(&value).is_some()),
            ReflectTypeRegistry::Func(func_registry) => (false, false, func_registry.get(&value).is_some()),
            ReflectTypeRegistry::AppFunc(func_registry) => (false, true, func_registry.read().get(&value).is_some()),
        };
        match retrieved {
            (true, false, true) => Ok(TypeRegistryIdentifier::Type(TypeRegistryTypeIdentifier(value))),
            (true, true, true) => Ok(TypeRegistryIdentifier::AppType(TypeRegistryTypeIdentifier(value))),
            (false, false, true) => Ok(TypeRegistryIdentifier::Func(TypeRegistryFuncIdentifier(value))),
            (false, true, true) => Ok(TypeRegistryIdentifier::AppFunc(TypeRegistryFuncIdentifier(value))),
            _ => Err(DynResolutionError::NotInRegistry(value.to_owned())),
        }
    }
}
