#![deny(unsafe_code)]
pub mod bounded;
pub mod client;
pub mod permit;
pub mod server;
pub mod live;
pub mod live_client;
#[cfg(windows)]
#[allow(unsafe_code)]
mod pipe_identity;
pub mod state;
pub use client::{CadGateway, McpGateway};
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
