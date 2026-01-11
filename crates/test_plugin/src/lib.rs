//! This crate extends the Cranium game AI library with a plugin used to standardize testing the library itself.
//! 
//! 
#![no_std]

mod helpers;
mod plugin;

pub use helpers::*;
pub use plugin::CraniumTestPlugin;
