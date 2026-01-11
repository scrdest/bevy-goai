#![doc = include_str!("../README.md")]
#![no_std]

pub use cortex_core::*;

pub mod prelude {
    pub use cortex_core::*;
    pub use cortex_core::types::*;
    pub use cortex_core::actions::AcceptsActionHandlerRegistrations;
    pub use cortex_core::actions::ActionPickCallback;
    pub use cortex_core::actions::ActionHandlerInputs;
    pub use cortex_core::action_runtime::TickBasedActionTrackerPlugin;
    pub use cortex_core::action_runtime::UserDefaultActionTrackerSpawnConfig;
    pub use cortex_core::ai::AIController;
    pub use cortex_core::considerations::AcceptsConsiderationRegistrations;
    pub use cortex_core::context_fetchers::AcceptsContextFetcherRegistrations;
    pub use cortex_core::curves::AcceptsCurveRegistrations;
    pub use cortex_core::events::AiDecisionRequested;
    pub use cortex_core::pawn::Pawn;

    #[cfg(any(feature = "bevy_plugin", feature = "testing"))]
    pub use cortex_bevy_plugin::CortexPlugin;

    #[cfg(any(feature = "cortex-test-plugin", feature = "testing"))]
    pub use cortex_test_plugin::CortexTestPlugin;

    #[cfg(feature = "actionset_loader")]
    pub use cortex_actionset_loader;
}
