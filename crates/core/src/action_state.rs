use bevy::reflect::Reflect;

#[cfg(any(feature = "actionset_loader"))]
use serde::{Deserialize, Serialize};

/// A lifecycle marker for ActionTrackers to indicate what the status of the tracked Action is. 
/// 
/// This is a state machine, more or less, with three layers:
/// 1) Initial
/// 2) Progressed
/// 3) Terminal
/// 
/// *As a general rule*, the progression through those layers should be non-increasing, i.e.:
/// - Terminal States should never change at all once reached, 
/// - Progressed states should only become Terminal or different Progressed States, and
/// - Initial states can become any other State.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Reflect)]
#[cfg_attr(any(feature = "actionset_loader"), derive(Serialize, Deserialize))]
pub enum ActionState {
    // Note that we are NOT implementing Default for this on purpose; 
    // the default state is domain-specific (though probably just Ready most of the time).

    /// Initial state. Planned but not started and cannot start yet - waiting on something (likely other Actions).
    Queued, 
    /// Initial state. Can start now, hadn't done literally anything yet though.
    Ready, 
    
    /// Progressed state. Started but didn't finish yet, will continue.
    Running, 
    /// Progressed state. Started and can continue, but has been put on hold for now.
    Paused, 
    
    /// Terminal state. Did all it was supposed to and is no longer needed, yaaay.
    Succeeded, 
    
    /// Terminal state. 
    /// We gave up due to getting stuck/timeout/etc., naaay. 
    /// Generally implies the action should not be retried as-is 
    /// and may trigger failure callbacks.
    Failed, 
    
    /// Terminal state. 
    /// We gave up because of an 'external' decision that we should, 
    /// and NOT because something was wrong with the execution. 
    /// The Action may well be still valid, we just stopped pursuing it. 
    /// You can think of it as 'Paused, but forever'. 
    Cancelled, 
}

impl ActionState {
    /// A shorthand for checking if an Action is in one of the Initial states (e.g. Ready).
    pub fn is_initial(&self) -> bool {
        match self {
            Self::Queued => true,
            Self::Ready => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action is in one of the Progressed states (e.g. Running).
    pub fn is_progressed(&self) -> bool {
        match self {
            Self::Running => true,
            Self::Paused => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action is in one of the Terminal states (e.g. Succeeded).
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Succeeded => true,
            Self::Failed => true,
            Self::Cancelled => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action should be processed/'ticked'. If false, we can skip it.
    pub fn should_process(&self) -> bool {
        match self {
            Self::Ready => true,
            Self::Running => true,
            _ => false,
        }
    }
}
