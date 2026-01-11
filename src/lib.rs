#![doc = include_str!("../README.md")]
#![no_std]

pub use cortex_ai_core::*;

pub mod prelude {
    pub use cortex_ai_core::*;
    pub use cortex_ai_core::types::*;
    pub use cortex_ai_core::actions::AcceptsActionHandlerRegistrations;
    pub use cortex_ai_core::actions::ActionPickCallback;
    pub use cortex_ai_core::actions::ActionHandlerInputs;
    pub use cortex_ai_core::action_runtime::TickBasedActionTrackerPlugin;
    pub use cortex_ai_core::action_runtime::UserDefaultActionTrackerSpawnConfig;
    pub use cortex_ai_core::ai::AIController;
    pub use cortex_ai_core::considerations::AcceptsConsiderationRegistrations;
    pub use cortex_ai_core::context_fetchers::AcceptsContextFetcherRegistrations;
    pub use cortex_ai_core::curves::AcceptsCurveRegistrations;
    pub use cortex_ai_core::events::AiDecisionRequested;
    pub use cortex_ai_core::pawn::Pawn;

    #[cfg(any(feature = "bevy_plugin", feature = "testing"))]
    pub use cortex_ai_bevy_plugin::CortexPlugin;

    #[cfg(any(feature = "cortex-ai-test-plugin", feature = "testing"))]
    pub use cortex_ai_test_plugin::CortexTestPlugin;

    #[cfg(feature = "actionset_loader")]
    pub use cortex_actionset_loader;
}
