/* 
This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. 
If a copy of the MPL was not distributed with this file, 
You can obtain one at https://mozilla.org/MPL/2.0/. 
*/

//! This crate extends the Cranium game AI library with a solution for loading `ActionSets` from 
//! JSON data from any available Bevy [`AssetSource`](https://docs.rs/bevy/latest/bevy/asset/io/struct.AssetSource.html). 
//! 
//! For stock Bevy, this includes in-memory, local filesystem, or a web URLs, 
//! depending on the enabled features and the platform you are building for.
//! 
//! Note that other Bevy libraries and your own custom code may extend this 
//! with additional AssetSources.
#![no_std]

mod loader;

pub use loader::*;
