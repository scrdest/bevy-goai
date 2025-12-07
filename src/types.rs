//! Type aliases and 'abstracting' newtypes.

/// Type alias to make it easier to switch out what datatypes are used for Action Keys. 
/// Action Keys are effectively IDs, so they do not need to be human-readable.
pub type ActionKey = String;

pub type ActionScore = f32;


