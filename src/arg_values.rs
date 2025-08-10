use std::{collections::HashMap, fmt::Debug};
use bevy::reflect::{Reflect};
use enum_delegate;
use serde::{Serialize, Deserialize};


#[enum_delegate::register]
trait IsPrimitiveContextValue {}

impl IsPrimitiveContextValue for bool {}
impl IsPrimitiveContextValue for i32 {}
impl IsPrimitiveContextValue for f32 {}
impl IsPrimitiveContextValue for String {}


#[derive(Serialize, Deserialize, Reflect, Clone, Debug)]
#[enum_delegate::implement(IsPrimitiveContextValue)]
enum PrimitiveContextValue {
    Bool(bool),
    I32(i32),
    F32(f32),
    String(String),
}

#[enum_delegate::register]
trait IsContextValue {}

impl IsContextValue for &str {}
impl<T: IsPrimitiveContextValue> IsContextValue for T {}
impl<T: IsPrimitiveContextValue> IsContextValue for Vec<T> {}
impl<V: IsPrimitiveContextValue> IsContextValue for HashMap<String, V> {}

#[derive(Serialize, Deserialize, Reflect, Clone, Debug)]
#[enum_delegate::implement(IsContextValue)]
pub enum ContextValue{
    Bool(bool),
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
}

