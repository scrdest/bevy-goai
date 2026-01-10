use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::types::CortexKvMap;

type MemoryEntry = serde_json::Value;
type MemoryMap = CortexKvMap<String, (MemoryEntry, Timer)>;


#[derive(Component, Serialize, Deserialize)]
pub struct Memories(MemoryMap);

