//! This crate provides tooling to run Cranium as an "AI Server" for non-Bevy applications 
//! (or Bevy applications who want to keep their Worlds separate from the AI World).
//! 
//! This includes functions to create an ECS World, drive it tick-by-tick externally, 
//! update the state of the World to keep the AI decisions sane, etc.
#![no_std]

mod api;

pub use api::*;
