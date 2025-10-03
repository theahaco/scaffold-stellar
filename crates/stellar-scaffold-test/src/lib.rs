#![allow(
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc
)]
mod common;
mod registry;

// Re-export the main components that tests will commonly use
pub use common::{AssertExt, TestEnv, find_binary};
pub use registry::RegistryTest;

pub fn rpc_url() -> String {
    std::env::var("STELLAR_RPC_URL").unwrap_or_else(|_| "http://localhost:8000/rpc".to_string())
}

// If we need to expose any common test constants or utilities, they can go here
pub const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(240);
