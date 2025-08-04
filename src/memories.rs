use std::collections::HashMap;
use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json;

type MemoryEntry = serde_json::Value;
type MemoryMap = HashMap<String, (MemoryEntry, Timer)>;


#[derive(Component, Serialize, Deserialize)]
pub struct Memories(MemoryMap);

