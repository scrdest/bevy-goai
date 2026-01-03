//! This crate extends the Cortex game AI library with a solution for loading `ActionSets` from 
//! JSON data from any available Bevy [`AssetSource`](https://docs.rs/bevy/latest/bevy/asset/io/struct.AssetSource.html). 
//! 
//! For stock Bevy, this includes in-memory, local filesystem, or a web URLs, 
//! depending on the enabled features and the platform you are building for.
//! 
//! Note that other Bevy libraries and your own custom code may extend this 
//! with additional AssetSources.

mod loader;

pub use loader::*;
