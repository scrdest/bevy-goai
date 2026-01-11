//! This crate extends the Cortex game AI library with a plugin used to standardize testing the library itself.
//! 
//! 
#![no_std]

mod helpers;
mod plugin;

pub use helpers::*;
pub use plugin::CortexTestPlugin;
