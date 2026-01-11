use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::types::CraniumKvMap;

type MemoryEntry = serde_json::Value;
type MemoryMap = CraniumKvMap<String, (MemoryEntry, Timer)>;


#[derive(Component, Serialize, Deserialize)]
pub struct Memories(MemoryMap);

