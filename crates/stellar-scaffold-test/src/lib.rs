mod common;
mod registry;

// Re-export the main components that tests will commonly use
pub use common::{AssertExt, TestEnv, find_binary};
pub use registry::RegistryTest;

// If we need to expose any common test constants or utilities, they can go here
pub const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(240);
