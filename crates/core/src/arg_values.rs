use std::{collections::HashMap, fmt::Debug};
use bevy::reflect::{Reflect, PartialReflect};
use enum_delegate;
use serde::{Serialize, Deserialize};

use crate::type_registry::TypeRegistryIdentifier;


#[enum_delegate::register]
pub trait IsPrimitiveContextValue {}

impl IsPrimitiveContextValue for bool {}
impl IsPrimitiveContextValue for u32 {}
impl IsPrimitiveContextValue for i32 {}
impl IsPrimitiveContextValue for f32 {}
impl IsPrimitiveContextValue for String {}


#[derive(Serialize, Deserialize, Reflect, Clone, Debug)]
#[enum_delegate::implement(IsPrimitiveContextValue)]
enum PrimitiveContextValue {
    Bool(bool),
    U32(u32),
    I32(i32),
    F32(f32),
    String(String),
}

#[enum_delegate::register]
pub trait IsContextValue {}

// A pair of mutually exclusive marker traits for blanket impls.
// ContextValueIsOpaque <=> !ContextValueIsTransparent effectively, similar to how ?Sized works.
// Opaque means the ContextValue is stored as a PartialReflect object wrapping the actual value, 
//   so the user needs to cast down to the actual type manually - but we can put all sorts of magic in there.
// Transparent is simple to read, but more limited - the value must be explicitly supported as a GOAI type.
pub trait ContextValueIsOpaque: IsPrimitiveContextValue {}
pub trait ContextValueIsTransparent: IsPrimitiveContextValue {}

impl<T: IsPrimitiveContextValue> ContextValueIsTransparent for T {}

// Convenience - it's not really Serialize, but lets us avoid cloning into Strings
impl IsContextValue for &str {}

// Fixed-size, stack-ey, 'compound' versions of primitive types (plain, tuples, arrays, etc.)
impl<T: IsPrimitiveContextValue> IsContextValue for (T, T) {}
impl<T: IsPrimitiveContextValue> IsContextValue for (T, T, T) {}
impl<T: IsPrimitiveContextValue> IsContextValue for (T, T, T, T) {}
impl<T: IsPrimitiveContextValue, const N: usize> IsContextValue for [T; N] {}

// 'Heapey' types. This will necessary have to be somewhat constrained for my sanity.
// For now, mainly the classic DSs as seen in your JSONs, Pythons, and whatever.
impl<T: IsPrimitiveContextValue> IsContextValue for Vec<T> {}
impl<V: IsPrimitiveContextValue> IsContextValue for HashMap<String, V> {}

// God have mercy on our souls, object references.
impl<T: PartialReflect + ContextValueIsOpaque> IsContextValue for T {}

/// This is a generic wrapper for Some Reflect Value.
/// If you cannot squeeze it into a Context any other way, you can always Reflect it in and then back out.
/// This does have three important caveats, however:
/// 
/// 1) The input must be Reflect (unsurprisingly...).
/// 2) You must prove that you have registered it in your app's registry (by constructing the wrapper).
/// 3) The input should be safely 'truly deep-Clone-able'. 
/// 
/// Things like Arc<T> might be Clone, but are effectively shallow copies.
/// 'Reconstituting' types from Reflect might bypass such Clone implementations and lead to unexpected behavior. 
impl IsContextValue for TypeRegistryIdentifier {}


#[derive(Serialize, Deserialize, Reflect, Clone, Debug)]
#[enum_delegate::implement(IsContextValue)]
pub enum ContextValue{
    Bool(bool),
    U32(u32),
    I32(i32),
    F32(f32),
    String(String),
    VecBool(Vec<bool>),
    VecI32(Vec<i32>),
    VecF32(Vec<f32>),
    VecStr(Vec<String>),
    MapBool(HashMap<String, bool>),
    MapI32(HashMap<String, i32>),
    MapF32(HashMap<String, f32>),
    MapString(HashMap<String, String>),
    Opaque(TypeRegistryIdentifier),
}

