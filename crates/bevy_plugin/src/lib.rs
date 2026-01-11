//! This crate extends the Cortex game AI library with a plugin that streamlines the integration 
//! of Cortex into an existing Bevy application (the "native AI" integration style).
//! 
//! The plugin handles the vast majority of the basic gruntwork - setting up generic Resources, 
//! Observers and Systems that form the 'framework' part of the AI.
//! 
//! What's left for you to do after adding it in is registering any ContextFetchers, Considerations, 
//! and any other of your custom Systems that you want Cortex to use
#![no_std]

mod plugin;

pub use plugin::CortexPlugin;
