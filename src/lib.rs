#![doc = include_str!("../README.md")]

pub use cortex_core::*;

pub mod prelude {
    pub use cortex_core::*;
    pub use cortex_core::types::*;
    pub use cortex_core::considerations::AcceptsConsiderationRegistrations;
    pub use cortex_core::context_fetchers::AcceptsContextFetcherRegistrations;

    #[cfg(feature = "bevy_plugin")]
    pub use cortex_bevy_plugin::CortexPlugin;

    #[cfg(feature = "actionset_loader")]
    pub use cortex_actionset_loader;
}
