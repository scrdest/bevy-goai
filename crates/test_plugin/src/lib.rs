/* 
This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. 
If a copy of the MPL was not distributed with this file, 
You can obtain one at https://mozilla.org/MPL/2.0/. 
*/
//! This crate extends the Cranium game AI library with a plugin used to standardize testing the library itself.
//! 
//! 
#![no_std]

mod helpers;
mod plugin;

pub use helpers::*;
pub use plugin::CraniumTestPlugin;
